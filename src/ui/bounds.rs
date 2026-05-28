#[derive(Debug, Default, Clone)]
pub struct Rect {
    pub x1: f64,
    pub y1: f64,

    pub x2: f64,
    pub y2: f64,
}

impl Rect {
    pub fn from_xywh(x: f64, y: f64, w: f64, h: f64) -> Self {
        Rect {
            x1: x,
            y1: y,

            x2: x + w,
            y2: y + h,
        }
    }

    /// Returns a boolean indicating if the (x, y) is on or within the rect.
    pub fn contains(&self, at: (f64, f64)) -> bool {
        at.0 >= self.x1 && at.0 <= self.x2 && at.1 >= self.y1 && at.1 <= self.y2
    }
}
