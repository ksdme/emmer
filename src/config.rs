use anyhow::Context;
use skia_safe::{Color, Font, FontStyle, Paint};

pub struct Measure {
    /// The measure in the horizontal axis.
    pub x: f32,

    /// The measure in the vertical axis.
    pub y: f32,
}

pub struct StackConfig {
    /// The height by which each stacked card will be peeking outside the
    /// the previous card.
    pub peek: f32,

    /// The width by which each stacked card will be smaller than the
    /// previous card.
    pub inset: f32,

    /// The maximum number of notification cards that will be visible when the
    /// cards are stacked.
    pub max_count: usize,
}

pub struct SpreadConfig {
    /// The gap between each notification card when the cards are spread out.
    pub gap: f32,

    /// The maximum number of notification cards that will be visible when the
    /// cards are spread out of the stack.
    pub max_count: usize,
}

pub struct ThemeConfig {
    pub font_family: String,
}

// TODO: Support padding.
// TODO: Support spread on stack.
// TODO: Support font sizes.
/// Represents the global application configuration.
pub struct Config {
    /// The margin around the cards.
    pub margin: Measure,

    /// The width of a notification card.
    pub width: f32,

    /// The configuration of the notification stack.
    pub stack: StackConfig,

    /// The configuration of the notifications spread.
    pub spread: SpreadConfig,

    /// The theme of the notifications.
    pub theme: ThemeConfig,
}

/// A theme object with pre-computations.
pub struct Theme {
    pub title_font: Font,
    pub title_paint: Paint,

    pub body_font: Font,
    pub body_paint: Paint,
}

impl From<&ThemeConfig> for Theme {
    fn from(config: &ThemeConfig) -> Self {
        let font_mgr = skia_safe::FontMgr::default();

        let bold_face = font_mgr
            .match_family_style(&config.font_family, FontStyle::bold())
            .context("Could not resolve bold font face")
            .unwrap();
        let title_font = Font::from_typeface(bold_face, 14.);

        let mut title_paint = Paint::default();
        title_paint.set_color(Color::from_rgb(255, 255, 255));

        let normal_face = font_mgr
            .match_family_style(&config.font_family, FontStyle::normal())
            .context("Could not resolve normal font face")
            .unwrap();
        let body_font = Font::from_typeface(normal_face, 13.);

        let mut body_paint = Paint::default();
        body_paint.set_color(Color::from_rgb(255, 255, 255));

        Theme {
            title_font,
            title_paint,

            body_font,
            body_paint,
        }
    }
}
