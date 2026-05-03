pub struct Measure {
    /// The measure in the horizontal axis.
    pub x: f32,

    /// The measure in the vertical axis.
    pub y: f32,
}

pub struct Stack {
    /// The width by which each stacked card will be smaller than the
    /// previous card.
    pub inset: f32,

    /// The height by which each stacked card will be peeking outside the
    /// the previous card.
    pub peek: f32,

    /// The maximum number of notification cards that will be visible when the
    /// cards are stacked.
    pub max_count: u8,
}

pub struct Spread {
    /// The gap between each notification card when the cards are spread out.
    pub gap: f32,

    /// The maximum number of notification cards that will be visible when the
    /// cards are spread out of the stack.
    pub max_count: u8,
}

/// Represents the global application configuration.
pub struct Config {
    /// The margin around the cards.
    pub margin: Measure,

    /// The configuration of the notification stack.
    pub stack: Stack,

    /// The configuration of the notifications spread.
    pub spread: Spread,

    /// The width of a notification card.
    pub width: f32,
}
