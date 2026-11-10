mod computed;
mod file;
mod kdl;
mod theme;

use documented::DocumentedFields;
use struct_patch::Patch;

use kdl::Float;

pub use computed::ComputedConfig;
pub use file::default_path;
pub use theme::{BarTheme, ButtonTheme, NotificationTheme, SurfaceTheme, TextTheme, ThemeConfig};

#[derive(Debug, Clone, PartialEq, DocumentedFields, Patch)]
#[patch(attribute(derive(knus::Decode)))]
pub struct Insets {
    /// The measure in the horizontal axis.
    #[patch(name = "Float", attribute(knus(child, unwrap(argument))))]
    pub x: f64,

    /// The measure in the vertical axis.
    #[patch(name = "Float", attribute(knus(child, unwrap(argument))))]
    pub y: f64,
}

#[derive(Debug, PartialEq, DocumentedFields, Patch)]
#[patch(attribute(derive(knus::Decode)))]
pub struct StackConfig {
    /// The height by which each stacked card will be peeking outside the
    /// the previous card.
    #[patch(name = "Float", attribute(knus(child, unwrap(argument))))]
    pub peek: f64,

    /// The width by which each stacked card will be smaller than the
    /// previous card.
    #[patch(name = "Float", attribute(knus(child, unwrap(argument))))]
    pub inset: f64,

    /// The maximum number of notification cards that will be visible when the
    /// cards are stacked.
    #[patch(attribute(knus(child, unwrap(argument))))]
    pub max_count: usize,
}

#[derive(Debug, PartialEq, DocumentedFields, Patch)]
#[patch(attribute(derive(knus::Decode)))]
pub struct SpreadConfig {
    /// The gap between each notification card when the cards are spread out.
    #[patch(name = "Float", attribute(knus(child, unwrap(argument))))]
    pub gap: f64,

    /// The maximum number of notification cards that will be visible when the
    /// cards are spread out of the stack.
    #[patch(attribute(knus(child, unwrap(argument))))]
    pub max_count: usize,
}

// TODO: Support spread on stack.
/// Represents the global application configuration.
#[derive(Debug, DocumentedFields, Patch)]
#[patch(attribute(derive(knus::Decode)))]
pub struct Config {
    /// The width of a notification card.
    #[patch(name = "Float", attribute(knus(child, unwrap(argument))))]
    pub width: f64,

    /// The margin around the cards.
    #[patch(name = "InsetsPatch", attribute(knus(child)))]
    pub margin: Insets,

    /// The configuration of the notification stack.
    #[patch(name = "StackConfigPatch", attribute(knus(child)))]
    pub stack: StackConfig,

    /// The configuration of the notifications spread.
    #[patch(name = "SpreadConfigPatch", attribute(knus(child)))]
    pub spread: SpreadConfig,

    /// The theme of the notifications.
    #[patch(name = "theme::ThemeConfigPatch", attribute(knus(child)))]
    pub theme: ThemeConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: 320.,
            margin: Insets { x: 32., y: 32. },

            stack: StackConfig {
                peek: 8.,
                inset: 8.,
                max_count: 3,
            },

            spread: SpreadConfig {
                gap: 8.,
                max_count: 20,
            },

            theme: ThemeConfig::default(),
        }
    }
}
