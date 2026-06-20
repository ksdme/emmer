use std::time::{Duration, Instant};

/// Represents an action on the notification.
#[derive(Debug, Clone)]
pub struct Action {
    /// The human label for the action.
    label: String,

    /// The internal key of the action.
    key: String,
}

/// Represents an incoming notification item.
#[derive(Debug, Clone)]
pub struct Notification {
    /// The id of the notification.
    id: u32,

    /// The title.
    title: Option<String>,

    /// The content of the notification.
    body: Option<String>,

    /// The actions associated with the notification.
    actions: Vec<Action>,

    /// The timeout of the notification.
    expire_at: Option<Instant>,
}

impl Notification {
    // Build an instance of the notification from DBUS parameters.
    // https://specifications.freedesktop.org/notification/1.3/protocol.html
    pub fn from_dbus_parts(
        id: u32,
        summary: String,
        body: String,
        actions: Vec<String>,
        expire_timeout: i32,
    ) -> Self {
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

            actions: actions
                // https://specifications.freedesktop.org/notification/1.3/protocol.html#id-1.10.3.3.4
                .chunks(2)
                .filter_map(|action| match (action.first(), action.get(1)) {
                    (Some(key), Some(label)) => Some(Action {
                        key: key.to_string(),
                        label: label.to_string(),
                    }),
                    _ => None,
                })
                .collect(),

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

    pub fn actions(&self) -> &Vec<Action> {
        &self.actions
    }

    /// Returns a boolean indicating if the notification has expired.
    pub fn is_expired(&self) -> bool {
        self.expire_at
            .map(|at| Instant::now() >= at)
            .unwrap_or_default()
    }
}
