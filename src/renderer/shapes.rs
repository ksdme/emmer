use std::f64::consts::PI;

/// Pushes a rounded rectangle to the context.
pub fn rounded_rect(cx: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    cx.new_sub_path();
    cx.arc(x + r, y + r, r, PI, 3.0 * PI / 2.0);
    cx.arc(x + w - r, y + r, r, 3.0 * PI / 2.0, 2.0 * PI);
    cx.arc(x + w - r, y + h - r, r, 0.0, PI / 2.0);
    cx.arc(x + r, y + h - r, r, PI / 2.0, PI);
    cx.close_path();
}

// Push an optionally round rectangle to the context.
pub fn rect(cx: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: Option<f64>) {
    if let Some(r) = r {
        rounded_rect(cx, x, y, w, h, r);
    } else {
        cx.rectangle(x, y, w, h);
    }
}
