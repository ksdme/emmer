use pango::FontDescription;

use crate::{
    config::{Insets, kdl::settings},
    ui::renderables::Color,
};

settings! {
    /// The colors that everything falls back to.
    pub struct Colors {
        /// The color of the text.
        pub text: Color,

        /// The color of the secondary text, like the app name and the time.
        pub muted: Color,

        /// The color the notifications, the action buttons and the bar are
        /// filled with.
        pub background: Color,

        /// The color of the borders.
        pub border: Color,
    }

    /// The look of a piece of text.
    pub struct TextConfig {
        /// Overrides the font of the text.
        pub font: Option<FontDescription>,

        /// Overrides the color of the text.
        pub color: Option<Color>,
    }

    /// The look of a piece of text that is spaced out from the body.
    pub struct SpacedTextConfig {
        /// Overrides the font of the text.
        pub font: Option<FontDescription>,

        /// Overrides the color of the text.
        pub color: Option<Color>,

        /// The space between it and the body.
        pub gap: f64,
    }

    /// The look of the image of a notification.
    pub struct ImageConfig {
        /// The radius of the corners, which is capped to a third of the height
        /// of the image.
        pub radius: f64,

        /// The space between the image and the text.
        pub gap: f64,
    }

    /// The look of a notification.
    pub struct NotificationConfig {
        /// The space between the edges and the contents.
        pub padding: Insets,

        /// The radius of the corners.
        pub radius: f64,

        /// The width of the border, a zero width disables the border.
        pub border_width: f64,

        /// Overrides the background color.
        pub background: Option<Color>,

        /// Overrides the border color.
        pub border_color: Option<Color>,

        /// The summary line.
        pub title: SpacedTextConfig,

        /// The main text.
        pub body: TextConfig,

        /// The app name and the time, which use the muted color.
        pub footer: SpacedTextConfig,

        /// The image or the icon.
        pub image: ImageConfig,
    }

    /// The look of a clickable control.
    pub struct ControlConfig {
        /// The space between the edges and the labels.
        pub padding: Insets,

        /// The radius of the corners.
        pub radius: f64,

        /// The width of the border, a zero width disables the border.
        pub border_width: f64,

        /// The space that separates it from its neighbours.
        pub gap: f64,

        /// Overrides the background color.
        pub background: Option<Color>,

        /// Overrides the border color.
        pub border_color: Option<Color>,

        /// Overrides the font of the labels.
        pub font: Option<FontDescription>,

        /// Overrides the color of the labels.
        pub color: Option<Color>,
    }
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            text: Color::from_rgba_u8(255, 255, 255, 1.),
            muted: Color::from_rgba_u8(255, 255, 255, 0.35),
            background: Color::from_rgba_u8(52, 52, 52, 1.),
            border: Color::from_rgba_u8(102, 102, 102, 1.),
        }
    }
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            padding: Insets { x: 12., y: 12. },
            radius: 8.,
            border_width: 1.5,
            background: None,
            border_color: None,

            title: SpacedTextConfig {
                font: Some(FontDescription::from_string(
                    "JetBrainsMono Nerd Font Mono Bold 10",
                )),
                color: None,
                gap: 8.,
            },
            body: TextConfig {
                font: None,
                color: None,
            },
            footer: SpacedTextConfig {
                font: None,
                color: None,
                gap: 6.,
            },

            image: ImageConfig {
                radius: 6.,
                gap: 8.,
            },
        }
    }
}

impl ControlConfig {
    pub fn new(padding: Insets) -> Self {
        Self {
            padding,
            radius: 4.,
            border_width: 1.,
            gap: 8.,
            background: None,
            border_color: None,
            font: None,
            color: None,
        }
    }
}
