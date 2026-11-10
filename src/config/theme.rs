use documented::DocumentedFields;
use pango::FontDescription;
use struct_patch::Patch;

use crate::{
    config::{
        Insets, InsetsPatch,
        kdl::{Float, Font},
    },
    ui::renderables::Color,
};

/// The theme of a box shaped surface.
#[derive(Debug, Clone, PartialEq, DocumentedFields, Patch)]
#[patch(attribute(derive(knus::Decode)))]
pub struct SurfaceTheme {
    /// The color the surface is filled with.
    #[patch(attribute(knus(child, unwrap(argument))))]
    pub background: Color,

    /// The width of the border, a zero width disables the border.
    #[patch(name = "Float", attribute(knus(child, unwrap(argument))))]
    pub border_width: f64,

    /// The color of the border.
    #[patch(attribute(knus(child, unwrap(argument))))]
    pub border_color: Color,

    /// The radius of the corners.
    #[patch(name = "Float", attribute(knus(child, unwrap(argument))))]
    pub border_radius: f64,

    /// The space between the edges of the surface and its contents.
    #[patch(name = "InsetsPatch", attribute(knus(child)))]
    pub padding: Insets,
}

/// The theme of a piece of text.
#[derive(Debug, Clone, PartialEq, DocumentedFields, Patch)]
#[patch(attribute(derive(knus::Decode)))]
pub struct TextTheme {
    /// The font of the text.
    /// https://docs.gtk.org/Pango/type_func.FontDescription.from_string.html#description
    #[patch(name = "Font", attribute(knus(child, unwrap(argument))))]
    pub font: FontDescription,

    /// The color of the text.
    #[patch(attribute(knus(child, unwrap(argument))))]
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
#[derive(Debug, Clone, PartialEq, DocumentedFields, Patch)]
#[patch(attribute(derive(knus::Decode)))]
pub struct NotificationTheme {
    /// The theme of the card itself.
    #[patch(name = "SurfaceThemePatch", attribute(knus(child)))]
    pub surface: SurfaceTheme,

    /// The theme of the title.
    #[patch(name = "TextThemePatch", attribute(knus(child)))]
    pub title: TextTheme,

    /// The theme of the body.
    #[patch(name = "TextThemePatch", attribute(knus(child)))]
    pub body: TextTheme,

    /// The theme of the app name and the time in the footer.
    #[patch(name = "TextThemePatch", attribute(knus(child)))]
    pub meta: TextTheme,

    /// The radius of the corners of the image, which is capped to a third of
    /// the height of the image.
    #[patch(name = "Float", attribute(knus(child, unwrap(argument))))]
    pub image_radius: f64,

    /// The gap between the image and the text.
    #[patch(name = "Float", attribute(knus(child, unwrap(argument))))]
    pub image_gap: f64,

    /// The gap between the title and the body.
    #[patch(name = "Float", attribute(knus(child, unwrap(argument))))]
    pub title_gap: f64,

    /// The gap between the contents and the footer.
    #[patch(name = "Float", attribute(knus(child, unwrap(argument))))]
    pub meta_gap: f64,
}

/// The theme of an action button.
#[derive(Debug, Clone, PartialEq, DocumentedFields, Patch)]
#[patch(attribute(derive(knus::Decode)))]
pub struct ButtonTheme {
    /// The theme of the button itself.
    #[patch(name = "SurfaceThemePatch", attribute(knus(child)))]
    pub surface: SurfaceTheme,

    /// The theme of the label.
    #[patch(name = "TextThemePatch", attribute(knus(child)))]
    pub label: TextTheme,

    /// The gap between the buttons and from the notification card.
    #[patch(name = "Float", attribute(knus(child, unwrap(argument))))]
    pub gap: f64,
}

/// The theme of the dismiss-all bar.
#[derive(Debug, Clone, PartialEq, DocumentedFields, Patch)]
#[patch(attribute(derive(knus::Decode)))]
pub struct BarTheme {
    /// The theme of the bar itself.
    #[patch(name = "SurfaceThemePatch", attribute(knus(child)))]
    pub surface: SurfaceTheme,

    /// The theme of the labels.
    #[patch(name = "TextThemePatch", attribute(knus(child)))]
    pub label: TextTheme,
}

#[derive(Debug, PartialEq, DocumentedFields, Patch)]
#[patch(attribute(derive(knus::Decode)))]
pub struct ThemeConfig {
    /// The theme of the notification cards.
    #[patch(name = "NotificationThemePatch", attribute(knus(child)))]
    pub notification: NotificationTheme,

    /// The theme of the action buttons.
    #[patch(name = "ButtonThemePatch", attribute(knus(child)))]
    pub button: ButtonTheme,

    /// The theme of the dismiss-all bar.
    #[patch(name = "BarThemePatch", attribute(knus(child)))]
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
