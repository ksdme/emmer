/// Represents an incoming notification item.
#[derive(Debug, Clone)]
pub struct Notification {
    /// The id of the notification.
    pub id: u32,

    /// The title. Freedesktop calls it summary though.
    pub summary: Option<String>,

    /// The content of the notification.
    pub body: Option<String>,
}
