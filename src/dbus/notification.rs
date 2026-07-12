use std::{path::PathBuf, time::Instant};

use debug_ignore::DebugIgnore;

/// Represents an action on the notification.
#[derive(Debug, Clone)]
pub struct Action {
    /// The internal key of the action.
    key: String,

    /// The human label for the action.
    label: String,
}

impl Action {
    pub fn new(key: String, label: String) -> Self {
        Self { key, label }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

/// A named container for the image data tuple.
#[derive(Debug)]
pub struct ImageData {
    pub width: i32,
    pub height: i32,
    pub row_stride: i32,
    pub has_alpha: bool,
    pub bits_per_sample: i32,
    pub channels: i32,
    pub data: DebugIgnore<Vec<u8>>,
}

#[derive(Debug)]
pub enum ImageSource {
    File(PathBuf),
    Data(ImageData),
}

#[derive(Debug)]
pub enum Urgency {
    Low,
    Normal,
    Critical,
}

/// Represents an incoming notification item.
#[derive(Debug)]
pub struct Notification {
    /// The id of the notification.
    pub id: u32,

    /// The app that sent the notification.
    pub app_name: String,

    /// The title.
    pub title: Option<String>,

    /// The content of the notification.
    pub body: Option<String>,

    /// The image that should be attached to the notification.
    pub image: Option<ImageSource>,

    /// The urgency level associated of the notification.
    pub urgency: Urgency,

    /// The actions associated with the notification.
    pub actions: Vec<Action>,

    /// The timeout of the notification.
    pub expires_at: Option<Instant>,

    /// The datetime when the item was created.
    pub created_at: chrono::DateTime<chrono::Local>,
}
