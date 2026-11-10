mod computed;
mod file;
mod kdl;
mod theme;

use kdl::settings;
use pango::FontDescription;

use crate::ui::anchors::{HorizontalAnchor, VerticalAnchor};

pub use computed::{ComputedConfig, Surface, Text};
pub use file::default_path;
pub use theme::{Colors, ControlConfig, NotificationConfig};

settings! {
    pub struct Insets {
        /// The measure in the horizontal axis.
        pub x: f64,

        /// The measure in the vertical axis.
        pub y: f64,
    }

    pub struct AnchorConfig {
        /// The edge the cards are aligned to, one of "left", "center" or
        /// "right".
        pub horizontal: HorizontalAnchor,

        /// The edge the cards stack up from, one of "top" or "bottom".
        pub vertical: VerticalAnchor,
    }

    pub struct StackConfig {
        /// The height by which each stacked card will be peeking outside the
        /// the previous card.
        pub peek: f64,

        /// The width by which each stacked card will be smaller than the
        /// previous card.
        pub inset: f64,

        /// The maximum number of notification cards that will be visible when the
        /// cards are stacked.
        pub max_count: usize,
    }

    pub struct SpreadConfig {
        /// The gap between each notification card when the cards are spread out.
        pub gap: f64,

        /// The maximum number of notification cards that will be visible when the
        /// cards are spread out of the stack.
        pub max_count: usize,
    }

    pub struct TimeoutsConfig {
        /// The time in milliseconds after which a notification is dismissed, when
        /// the app leaves it to the server. A value of 0 never dismisses it.
        pub default: usize,

        /// Whether the default is used even when the app asks for a timeout.
        pub force_default: bool,
    }

    // TODO: Support spread on stack.
    /// Represents the global application configuration.
    pub struct Config {
        /// The width of a notification card.
        pub width: f64,

        /// The position of the cards on the screen.
        pub anchor: AnchorConfig,

        /// The margin around the cards.
        pub margin: Insets,

        /// The configuration of the notification stack.
        pub stack: StackConfig,

        /// The configuration of the notifications spread.
        pub spread: SpreadConfig,

        /// The time for which the notifications stay on screen.
        pub timeouts: TimeoutsConfig,

        /// The font that everything falls back to.
        /// https://docs.gtk.org/Pango/type_func.FontDescription.from_string.html#description
        pub font: FontDescription,

        /// The colors that everything falls back to.
        pub colors: Colors,

        /// The notification cards.
        pub notification: NotificationConfig,

        /// The action buttons below the notifications.
        pub actions: ControlConfig,

        /// The dismiss-all bar.
        pub bar: ControlConfig,
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: 320.,
            anchor: AnchorConfig {
                horizontal: HorizontalAnchor::Right,
                vertical: VerticalAnchor::Top,
            },
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

            timeouts: TimeoutsConfig {
                default: 0,
                force_default: false,
            },

            font: FontDescription::from_string("JetBrainsMono Nerd Font Mono 10"),
            colors: Colors::default(),

            notification: NotificationConfig::default(),
            actions: ControlConfig::new(Insets { x: 16., y: 8. }),
            bar: ControlConfig::new(Insets { x: 12., y: 6. }),
        }
    }
}
