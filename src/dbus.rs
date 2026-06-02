use std::sync::atomic::AtomicU32;

use log::error;
use smithay_client_toolkit::reexports::calloop::channel::{self};
use zbus::{interface, object_server::SignalEmitter, zvariant};

use crate::{UIMessage, notification::Notification};

// Represents a message from the UI thread to the server thread.
#[derive(Debug)]
pub enum ServerMessage {
    Dismiss { id: u32, reason: u32 },
    ActivationToken { id: u32, token: String },
    ActionInvoked { id: u32, key: String },
}

/// A dbus service for handling notification messages.
pub struct NotificationService {
    pub id_counter: AtomicU32,
    pub tx: channel::Sender<UIMessage>,
}

impl NotificationService {
    pub fn new(tx: channel::Sender<UIMessage>) -> Self {
        Self {
            id_counter: AtomicU32::default(),
            tx,
        }
    }
}

#[interface(name = "org.freedesktop.Notifications")]
impl NotificationService {
    fn notify(
        &self,
        _app_name: &str,
        _replaces_id: u32,
        _app_icon: &str,
        summary: &str,
        body: &str,
        actions: Vec<String>,
        _hints: std::collections::HashMap<String, zvariant::Value>,
        expire_timeout: i32,
    ) -> u32 {
        let id = self
            .id_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        match self.tx.send(UIMessage::Push(Notification::from_dbus_parts(
            id,
            summary.to_string(),
            body.to_string(),
            actions,
            expire_timeout,
        ))) {
            // TODO: How else to handle the error?
            Err(err) => {
                error!("could not send push message: {err}");
                0
            }
            Ok(_) => id,
        }
    }

    fn get_capabilities(&self) -> Vec<String> {
        vec!["body".into(), "actions".into()]
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
