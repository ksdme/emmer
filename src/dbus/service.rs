use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    time::Duration,
};

use anyhow::{Context, Error, Result, anyhow, bail};
use debug_ignore::DebugIgnore;
use log::error;
use smithay_client_toolkit::reexports::calloop::channel::{self};
use zbus::{
    interface,
    object_server::SignalEmitter,
    zvariant::{self},
};

use crate::{
    dbus::{
        image,
        notification::{self, ImageData, ImageSource, Urgency},
    },
    logged, sound,
    ui::app::UIMessage,
};

#[derive(Debug)]
pub enum CloseReason {
    Expired,
    Manual,
}

impl From<CloseReason> for u32 {
    fn from(v: CloseReason) -> Self {
        match v {
            CloseReason::Expired => 1,
            CloseReason::Manual => 2,
        }
    }
}

// Represents a message from the UI thread to the server thread.
#[derive(Debug)]
pub enum ServerMessage {
    Closed { id: u32, reason: CloseReason },
    ActivationToken { id: u32, token: String },
    ActionInvoked { id: u32, key: String },
}

/// A dbus service for handling notification messages.
pub struct NotificationService {
    pub id_counter: AtomicU32,
    pub tx: channel::Sender<UIMessage>,

    // zbus dispatches method calls on its own executor, so, the tokio runtime
    // has to be reached through a handle for scheduling blocking work.
    runtime: tokio::runtime::Handle,
    // Only one sound plays at a time, so that bursts of notifications do not pile
    // their sounds on top of each other.
    sound_busy: Arc<AtomicBool>,
}

impl NotificationService {
    /// Must be called from within a tokio runtime.
    pub fn new(runtime: tokio::runtime::Handle, tx: channel::Sender<UIMessage>) -> Self {
        Self {
            id_counter: AtomicU32::new(1),
            tx,
            runtime,
            sound_busy: Arc::default(),
        }
    }

    // Decodes the image on the blocking pool so that it does not stall the UI thread.
    async fn decode_image(&self, source: ImageSource) -> Option<image::Image> {
        logged!(
            self.runtime
                .spawn_blocking(move || image::decode(source, 64., Some(64.)))
                .await
                .context("Could not join image decoder")
                .and_then(|image| image.context("Could not decode image"))
        )
        .ok()
    }

    // https://specifications.freedesktop.org/notification/latest-single/#hints
    fn play_sound(&self, hints: &HashMap<String, zvariant::Value<'_>>) {
        let string = |key: &str| {
            hints
                .get(key)
                .and_then(|value| String::try_from(value).ok())
                .filter(|value| !value.is_empty())
        };

        // Supress the sound.
        let suppressed = hints
            .get("suppress-sound")
            .and_then(|value| bool::try_from(value).ok())
            .unwrap_or(false);
        if suppressed {
            return;
        }

        // Do we have a source?
        let file = string("sound-file");
        let name = string("sound-name");
        if file.is_none() && name.is_none() {
            return;
        }

        // Set the devive busy.
        if self.sound_busy.swap(true, Ordering::AcqRel) {
            return;
        }

        let sound_busy = self.sound_busy.clone();
        self.runtime.spawn_blocking(move || {
            // A missing "sound-file" falls back to the "sound-name".
            if let Some(path) = file
                .and_then(|file| file_from_uri(&file).ok())
                .or_else(|| name.and_then(|name| sound::find(&name)))
            {
                let _ = logged!(sound::play(&path).context("Could not play sound"));
            }

            sound_busy.store(false, Ordering::Release);
        });
    }
}

#[interface(name = "org.freedesktop.Notifications")]
impl NotificationService {
    async fn notify(
        &self,
        app_name: &str,
        _replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: Vec<String>,
        hints: HashMap<String, zvariant::Value<'_>>,
        expire_timeout: i32,
    ) -> u32 {
        let id = self
            .id_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // https://specifications.freedesktop.org/notification/latest-single/#id-1.6.4
        // 1. "image-data"
        let image_source = hints
            .get("image-data")
            .or_else(|| hints.get("image_data"))
            .and_then(|data| image_from_data(data).ok())
            // 2. "image-path"
            .or_else(|| {
                hints
                    .get("image-path")
                    // Deprecated < 1.1
                    .or_else(|| hints.get("image_path"))
                    .and_then(|uri| String::try_from(uri).ok())
                    .and_then(|uri| image_from_uri(&uri).ok())
            })
            // 3. app_icon parameter
            .or_else(|| image_from_uri(app_icon).ok())
            // 4. for compatibility reason, "icon_data"
            .or_else(|| {
                hints
                    .get("icon_data")
                    .and_then(|data| image_from_data(data).ok())
            });

        let image = match image_source {
            Some(source) => self.decode_image(source).await,
            None => None,
        };

        let urgency = match hints
            .get("urgency")
            .and_then(|value| u8::try_from(value).ok())
        {
            Some(0) => Urgency::Low,
            Some(1) => Urgency::Normal,
            Some(2) => Urgency::Critical,
            Some(_) | None => Urgency::Normal,
        };

        let actions = actions
            // https://specifications.freedesktop.org/notification/1.3/protocol.html#id-1.10.3.3.4
            .chunks(2)
            .filter_map(|action| match (action.first(), action.get(1)) {
                (Some(key), Some(label)) => Some(notification::Action::new(
                    key.to_string(),
                    label.to_string(),
                )),
                _ => None,
            })
            .collect();

        let expiry = if expire_timeout == 0 {
            None
        } else if expire_timeout < 0 {
            match urgency {
                // TODO: Maybe make the low and normal values configurable.
                Urgency::Low => Some(Duration::from_secs(5)),
                Urgency::Normal => Some(Duration::from_secs(10)),
                Urgency::Critical => None,
            }
        } else {
            Some(Duration::from_millis(expire_timeout as u64))
        };

        // Build an instance of the notification from DBUS parameters.
        // https://specifications.freedesktop.org/notification/1.3/protocol.html
        let notif = notification::Notification {
            id,
            app_name: app_name.to_string(),

            title: if summary.is_empty() {
                None
            } else {
                Some(summary.to_string())
            },
            body: if body.is_empty() {
                None
            } else {
                Some(body.to_string())
            },
            image,

            urgency,
            actions,

            expiry,
            created_at: chrono::Local::now(),
        };

        match self.tx.send(UIMessage::Push(notif)) {
            // TODO: How else to handle the error?
            Err(err) => {
                error!("could not send push message: {err}");
                0
            }
            Ok(_) => {
                // We are calling this from here instead of the other UI thread so
                // we do not have to create another runtime on that thread, especially since
                // it uses calloop.
                self.play_sound(&hints);
                id
            }
        }
    }

    fn get_capabilities(&self) -> Vec<String> {
        vec!["body".into(), "actions".into(), "sound".into()]
    }

    fn get_server_information(&self) -> (String, String, String, String) {
        ("emmer".into(), "me".into(), "1.0".into(), "1.2".into())
    }

    /// A signal that represents that notification has been closed either because it
    /// timed out or because the user dismissed it.
    #[zbus(signal)]
    pub async fn notification_closed(
        signal_emitter: &SignalEmitter<'_>,
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;

    /// A signal to pass the activation token that the target application can use to
    /// change focus.
    #[zbus(signal)]
    pub async fn activation_token(
        signal_emitter: &SignalEmitter<'_>,
        id: u32,
        activation_token: String,
    ) -> zbus::Result<()>;

    /// A signal to indicate that a specific action was invoked on the notification
    /// item.
    #[zbus(signal)]
    pub async fn action_invoked(
        signal_emitter: &SignalEmitter<'_>,
        id: u32,
        action_key: String,
    ) -> zbus::Result<()>;
}

// iiibiiay
// https://specifications.freedesktop.org/notification/latest-single/#icons-and-images-formats
type ImageDataTuple = (i32, i32, i32, bool, i32, i32, Vec<u8>);

impl TryInto<ImageData> for ImageDataTuple {
    type Error = Error;

    fn try_into(self) -> Result<ImageData, Self::Error> {
        if self.4 != 8 {
            bail!("bits_per_sample must be 8")
        }

        if self.3 && self.5 != 4 {
            bail!("has_alpha requires 4 channels")
        }

        if !self.3 && self.5 != 3 {
            bail!("!has_alpha requires 3 channels")
        }

        Ok(ImageData {
            width: self.0,
            height: self.1,

            has_alpha: self.3,
            row_stride: self.2,
            bits_per_sample: self.4,
            channels: self.5,

            data: DebugIgnore(self.6),
        })
    }
}

/// Constructs an image from pixel data.
fn image_from_data(data: &zvariant::Value) -> Result<ImageSource> {
    Ok(ImageSource::Data(
        ImageDataTuple::try_from(data)
            .context("Could not parse data tuple")
            .and_then(|data| data.try_into().context("Could not parse data structure"))?,
    ))
}

/// Constructs an image from the file:// uri.
fn image_from_uri(uri: &str) -> Result<ImageSource> {
    file_from_uri(uri).map(ImageSource::File)
}

/// Resolves a file:// uri, or an absolute path, into an existing file.
fn file_from_uri(uri: &str) -> Result<PathBuf> {
    let path = if uri.starts_with('/') {
        PathBuf::from(uri)
    } else {
        let uri = url::Url::parse(uri).context("Could not parse uri")?;
        if uri.scheme() != "file" {
            bail!("Unsupported scheme")
        }

        uri.to_file_path()
            .map_err(|_| anyhow!("Unsupported path"))?
    };

    // Checking for the file here itself makes the fallback ladders at the call
    // sites simpler.
    if !path.is_file() {
        bail!("Path does not exist or is not a file")
    }

    Ok(path)
}
