use std::time::{Duration, Instant};

/// Represents an incoming notification item.
#[derive(Debug, Clone)]
pub struct Notification {
    /// The id of the notification.
    id: u32,

    /// The title.
    title: Option<String>,

    /// The content of the notification.
    body: Option<String>,

    /// The timeout of the notification.
    expire_at: Option<Instant>,
}

impl Notification {
    // Build an instance of the notification from DBUS parameters.
    // https://specifications.freedesktop.org/notification/1.3/protocol.html
    pub fn from_dbus_parts(id: u32, summary: String, body: String, expire_timeout: i32) -> Self {
        Self {
            id,

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

            expire_at: if expire_timeout == 0 {
                None
            } else if expire_timeout < 0 {
                Some(Instant::now() + Duration::from_secs(30))
            } else {
                Some(Instant::now() + Duration::from_millis(expire_timeout as u64))
            },
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn body(&self) -> Option<&str> {
        self.body.as_deref()
    }

    /// Returns a boolean indicating if the notification has expired.
    pub fn is_expired(&self) -> bool {
        self.expire_at
            .map(|at| Instant::now() >= at)
            .unwrap_or_default()
    }
}
