use pango::FontDescription;

use crate::ui::{
    anchors::{Anchor, HorizontalAnchor, VerticalAnchor},
    renderables::Color,
};

#[derive(Debug, Clone)]
pub struct Insets {
    /// The measure in the horizontal axis.
    pub x: f64,

    /// The measure in the vertical axis.
    pub y: f64,
}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct SpreadConfig {
    /// The gap between each notification card when the cards are spread out.
    pub gap: f64,

    /// The maximum number of notification cards that will be visible when the
    /// cards are spread out of the stack.
    pub max_count: usize,
}

/// The theme of a box shaped surface.
#[derive(Debug, Clone)]
pub struct SurfaceTheme {
    /// The color the surface is filled with.
    pub background: Color,

    /// The width of the border, a zero width disables the border.
    pub border_width: f64,

    /// The color of the border.
    pub border_color: Color,

    /// The radius of the corners.
    pub border_radius: f64,

    /// The space between the edges of the surface and its contents.
    pub padding: Insets,
}

/// The theme of a piece of text.
#[derive(Debug, Clone)]
pub struct TextTheme {
    /// The font of the text.
    /// https://docs.gtk.org/Pango/type_func.FontDescription.from_string.html#description
    pub font: FontDescription,

    /// The color of the text.
    pub color: Color,
}

impl TextTheme {
    pub fn new(font: &str, color: Color) -> Self {
        Self {
            font: FontDescription::from_string(font),
            color,
        }
    }
}

/// The theme of a notification card.
#[derive(Debug, Clone)]
pub struct NotificationTheme {
    /// The theme of the card itself.
    pub surface: SurfaceTheme,

    /// The theme of the title.
    pub title: TextTheme,

    /// The theme of the body.
    pub body: TextTheme,

    /// The theme of the app name and the time in the footer.
    pub meta: TextTheme,

    /// The radius of the corners of the image, which is capped to a third of
    /// the height of the image.
    pub image_radius: f64,

    /// The gap between the image and the text.
    pub image_gap: f64,

    /// The gap between the title and the body.
    pub title_gap: f64,

    /// The gap between the contents and the footer.
    pub meta_gap: f64,
}

/// The theme of an action button.
#[derive(Debug, Clone)]
pub struct ButtonTheme {
    /// The theme of the button itself.
    pub surface: SurfaceTheme,

    /// The theme of the label.
    pub label: TextTheme,

    /// The gap between the buttons and from the notification card.
    pub gap: f64,
}

/// The theme of the dismiss-all bar.
#[derive(Debug, Clone)]
pub struct BarTheme {
    /// The theme of the bar itself.
    pub surface: SurfaceTheme,

    /// The theme of the labels.
    pub label: TextTheme,
}

#[derive(Debug)]
pub struct ThemeConfig {
    /// The theme of the notification cards.
    pub notification: NotificationTheme,

    /// The theme of the action buttons.
    pub button: ButtonTheme,

    /// The theme of the dismiss-all bar.
    pub bar: BarTheme,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        let font = "JetBrainsMono Nerd Font Mono 10";
        let bold_font = "JetBrainsMono Nerd Font Mono Bold 10";

        let background = Color::from_rgba_u8(52, 52, 52, 1.);
        let border_color = Color::from_rgba_u8(102, 102, 102, 1.);
        let text_color = Color::from_rgba_u8(255, 255, 255, 1.);

        let text = TextTheme::new(font, text_color);

        Self {
            notification: NotificationTheme {
                surface: SurfaceTheme {
                    background,
                    border_width: 1.5,
                    border_color,
                    border_radius: 8.,
                    padding: Insets { x: 12., y: 12. },
                },

                title: TextTheme::new(bold_font, text_color),
                body: text.clone(),
                meta: TextTheme::new(font, text_color.with_alpha(0.35)),

                image_radius: 6.,

                image_gap: 8.,
                title_gap: 8.,
                meta_gap: 6.,
            },

            button: ButtonTheme {
                surface: SurfaceTheme {
                    background,
                    border_width: 1.,
                    border_color,
                    border_radius: 4.,
                    padding: Insets { x: 16., y: 8. },
                },

                label: text.clone(),

                gap: 8.,
            },

            bar: BarTheme {
                surface: SurfaceTheme {
                    background,
                    border_width: 1.,
                    border_color,
                    border_radius: 4.,
                    padding: Insets { x: 12., y: 6. },
                },

                label: text,
            },
        }
    }
}

// TODO: Support spread on stack.
/// Represents the global application configuration.
#[derive(Debug)]
pub struct Config {
    /// The width of a notification card.
    pub width: f64,

    /// The margin around the cards.
    pub margin: Insets,

    /// The configuration of the notification stack.
    pub stack: StackConfig,

    /// The configuration of the notifications spread.
    pub spread: SpreadConfig,

    /// The theme of the notifications.
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

/// Represents a prepared configuration.
#[derive(Debug)]
pub struct ComputedConfig {
    pub debug_mode: bool,

    pub width: f64,
    pub margin: Insets,

    pub anchors: (HorizontalAnchor, VerticalAnchor),

    pub stack: StackConfig,
    pub spread: SpreadConfig,

    pub theme: ThemeConfig,
    pub font_map: pango::FontMap,
}

impl ComputedConfig {
    pub fn new(debug_mode: bool, config: Config) -> Self {
        Self {
            debug_mode,

            anchors: (HorizontalAnchor::Right, VerticalAnchor::Bottom),

            width: config.width,
            margin: config.margin,

            stack: config.stack,
            spread: config.spread,

            theme: config.theme,
            font_map: pangocairo::FontMap::default(),
        }
    }

    pub fn anchor(&self, l_w: f64, l_h: f64) -> Anchor {
        Anchor {
            horizontal: self.anchors.0,
            vertical: self.anchors.1,

            layer_width: l_w,
            layer_height: l_h,
        }
    }
}
