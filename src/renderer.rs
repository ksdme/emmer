use std::f64::consts::PI;

/// Represents a color in cairo as ARGB (0..1).
/// TODO: These should be single byte values.
pub struct Color(pub f64, pub f64, pub f64, pub f64);

/// Represents a border configuration.
pub struct Border {
    pub color: Color,
    pub width: f32,
}

/// Represents a drop shadow configuration.
pub struct Shadow {
    pub offset: (f32, f32),
    pub blur: f32,
    pub alpha: f32,
    pub color: Color,
}

/// Represents a blank rounded card.
pub struct Card {
    pub bg: Color,
    pub border: Option<Border>,
    pub radius: Option<f32>,
    pub shadow: Option<Shadow>,
}

impl Card {
    pub fn draw(&self, cx: cairo::Context, x: f64, y: f64, w: f64, h: f64) {
        // Add base rectangle path.
        if let Some(radius) = self.radius {
            rounded_rect(&cx, x, y, w, h, radius.into());
        } else {
            cx.rectangle(x, y, w, h);
        }

        // Add background.
        let bg = &self.bg;
        cx.set_source_rgba(bg.0, bg.1, bg.2, bg.3);
        cx.fill_preserve().expect("cairo fill");

        // Add the border.
        if let Some(border) = &self.border {
            cx.set_line_width(border.width.into());

            let color = &border.color;
            cx.set_source_rgba(color.0, color.1, color.2, color.3);

            cx.stroke_preserve().expect("cairo stroke");
        }
    }
}

/// Pushes a rounded rectangle to the context.
pub fn rounded_rect(cx: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    cx.new_sub_path();
    cx.arc(x + r, y + r, r, PI, 3.0 * PI / 2.0);
    cx.arc(x + w - r, y + r, r, 3.0 * PI / 2.0, 2.0 * PI);
    cx.arc(x + w - r, y + h - r, r, 0.0, PI / 2.0);
    cx.arc(x + r, y + h - r, r, PI / 2.0, PI);
    cx.close_path();
}
