#[derive(Debug)]
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
