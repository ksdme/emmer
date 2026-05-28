use pango::FontDescription;

#[derive(Debug)]
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

#[derive(Debug)]
pub struct ThemeConfig {
    /// The font description of the title.
    /// https://docs.gtk.org/Pango/type_func.FontDescription.from_string.html#description
    pub title_font_description: String,

    /// The font description of the body.
    /// https://docs.gtk.org/Pango/type_func.FontDescription.from_string.html#description
    pub body_font_description: String,
}

// TODO: Support padding.
// TODO: Support spread on stack.
// TODO: Support font sizes.
/// Represents the global application configuration.
#[derive(Debug)]
pub struct Config {
    /// The width of a notification card.
    pub width: f64,

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
    pub font_map: pango::FontMap,
    pub title_font_description: FontDescription,
    pub body_font_description: FontDescription,
}

impl From<ThemeConfig> for Theme {
    fn from(config: ThemeConfig) -> Self {
        Self {
            font_map: pangocairo::FontMap::default(),
            title_font_description: FontDescription::from_string(&config.title_font_description),
            body_font_description: FontDescription::from_string(&config.body_font_description),
        }
    }
}

/// Represents a prepared configuration.
#[derive(Debug)]
pub struct ComputedConfig {
    pub width: f64,

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
