use anyhow::{Context, Result};

use crate::ui::renderables::{Color, Rect, rounded_sub_path};

/// Represents a generic box.
#[derive(Debug)]
pub struct Renderable;

impl Renderable {
    pub fn new() -> Self {
        Self {}
    }

    pub fn render(
        &self,
        cr: &cairo::Context,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        bg: Color,
        border_width: f64,
        border_color: Option<Color>,
        border_radius: f64,
    ) -> Result<Rect> {
        let r = border_radius;

        // Path.
        cr.new_path();
        if r == 0. {
            cr.rectangle(x, y, w, h);
        } else {
            rounded_sub_path(cr, x, y, w, h, r);
        }

        // Background.
        cr.set_source_rgba(bg.r, bg.g, bg.b, bg.a);
        cr.fill_preserve()
            .context("Could not fill shape on main surface")?;

        // Border.
        if border_width > 0. {
            cr.set_line_width(border_width);

            let color = border_color.unwrap_or_default();
            cr.set_source_rgba(color.r, color.g, color.b, color.a);

            cr.stroke_preserve()
                .context("Could not stroke main surface path")?;
        }

        Ok(Rect::from_xywh(x, y, w, h))
    }
}
