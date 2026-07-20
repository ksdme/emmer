/// The horizontal anchor position.
#[derive(Debug, Clone, Copy)]
pub enum HorizontalAnchor {
    Left,
    Center,
    Right,
}

/// The vertical anchor position.
#[derive(Debug, Clone, Copy)]
pub enum VerticalAnchor {
    Top,
    Bottom,
}

/// Represents tha anchor for the stack.
#[derive(Debug, Clone, Copy)]
pub struct Anchor {
    pub horizontal: HorizontalAnchor,
    pub vertical: VerticalAnchor,

    pub layer_width: f64,
    pub layer_height: f64,
}
