use skia_safe::{
    Color, FontMgr, Paint,
    textlayout::{FontCollection, TextStyle},
};

#[derive(Debug)]
pub struct Insets {
    /// The measure in the horizontal axis.
    pub x: f32,

    /// The measure in the vertical axis.
    pub y: f32,
}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct SpreadConfig {
    /// The gap between each notification card when the cards are spread out.
    pub gap: f32,

    /// The maximum number of notification cards that will be visible when the
    /// cards are spread out of the stack.
    pub max_count: usize,
}

#[derive(Debug)]
pub struct ThemeConfig {
    /// The font family for all content.
    pub font_family: String,
}

// TODO: Support padding.
// TODO: Support spread on stack.
// TODO: Support font sizes.
/// Represents the global application configuration.
#[derive(Debug)]
pub struct Config {
    /// The width of a notification card.
    pub width: f32,

    /// The margin around the cards.
    pub margin: Insets,

    /// The padding inside the cards.
    pub padding: Insets,

    /// The configuration of the notification stack.
    pub stack: StackConfig,

    /// The configuration of the notifications spread.
    pub spread: SpreadConfig,

    /// The theme of the notifications.
    pub theme: ThemeConfig,
}

#[derive(Debug)]
pub struct Theme {
    pub font_collection: FontCollection,

    pub title_style: TextStyle,
    pub body_style: TextStyle,
}

impl From<ThemeConfig> for Theme {
    fn from(config: ThemeConfig) -> Self {
        let mut font_collection = FontCollection::new();
        font_collection.set_default_font_manager(FontMgr::default(), None);

        let mut text_paint = Paint::default();
        text_paint.set_color(Color::from_rgb(255, 255, 255));

        let mut title_style = TextStyle::default();
        title_style
            .set_font_families(&[&config.font_family.clone()])
            .set_font_size(14.)
            .set_foreground_paint(&text_paint);

        let mut body_style = TextStyle::default();
        body_style
            .set_font_families(&[&config.font_family])
            .set_font_size(13.)
            .set_foreground_paint(&text_paint);

        Self {
            font_collection,
            title_style,
            body_style,
        }
    }
}

/// Represents a prepared configuration.
#[derive(Debug)]
pub struct ComputedConfig {
    pub width: f32,

    pub margin: Insets,
    pub padding: Insets,

    pub stack: StackConfig,
    pub spread: SpreadConfig,

    pub theme: Theme,
}

impl From<Config> for ComputedConfig {
    fn from(config: Config) -> Self {
        Self {
            width: config.width,

            margin: config.margin,
            padding: config.padding,

            stack: config.stack,
            spread: config.spread,

            theme: Theme::from(config.theme),
        }
    }
}
