/// Represents a color.
#[derive(Debug, Default, Clone, Copy)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_rgba_u8(r: u8, g: u8, b: u8, a: f64) -> Self {
        Self {
            r: r as f64 / 255.,
            g: g as f64 / 255.,
            b: b as f64 / 255.,
            a,
        }
    }
}

/// Represents a box, usually used to represent hitboxes.
#[derive(Debug, Default, Clone, Copy)]
pub struct Rect {
    pub x1: f64,
    pub y1: f64,

    pub x2: f64,
    pub y2: f64,
}

impl Rect {
    pub fn from_xywh(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self {
            x1: x,
            y1: y,
            x2: (x + w),
            y2: (y + h),
        }
    }

    pub fn w(&self) -> f64 {
        self.x2 - self.x1
    }

    pub fn h(&self) -> f64 {
        self.y2 - self.y1
    }

    /// Returns a boolean indicating if the (x, y) is on or within the rect.
    #[inline]
    pub fn contains(&self, at: (f64, f64)) -> bool {
        at.0 >= self.x1 && at.0 <= self.x2 && at.1 >= self.y1 && at.1 <= self.y2
    }
}

pub mod card;
pub mod notification;
pub mod text;
