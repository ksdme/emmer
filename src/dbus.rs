use std::sync::atomic::AtomicU32;

use log::error;
use smithay_client_toolkit::reexports::calloop::channel::{self};
use zbus::{interface, zvariant};

use crate::{UIMessage, notification::Notification};

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
        _actions: Vec<String>,
        _hints: std::collections::HashMap<String, zvariant::Value>,
        _expire_timeout: i32,
    ) -> u32 {
        let id = self
            .id_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        match self.tx.send(UIMessage::Push(Notification {
            id,
            body: body.to_string(),
            summary: summary.to_string(),
        })) {
            // TODO: How else to handle the error?
            Err(err) => {
                error!(target: "zbusloop", "could not send push message: {err}");
                0
            }
            Ok(_) => id,
        }
    }

    fn get_capabilities(&self) -> Vec<String> {
        vec!["body".into()]
    }

    fn get_server_information(&self) -> (String, String, String, String) {
        ("emmer".into(), "me".into(), "1.0".into(), "1.2".into())
    }
}
