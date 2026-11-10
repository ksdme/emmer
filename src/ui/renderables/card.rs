use anyhow::{Context, Result};

use crate::{
    config::Surface,
    ui::renderables::{Color, Rect, rounded_sub_path},
};

/// The style of a generic box.
#[derive(Debug, Default, Clone, Copy)]
pub struct Style {
    pub bg: Color,
    pub border_width: f64,
    pub border_color: Color,
    pub border_radius: f64,
}

impl Style {
    /// Returns the style of a surface faded by opacity.
    pub fn from_surface(surface: &Surface, opacity: f64) -> Self {
        Self {
            bg: surface.background.faded(opacity),
            border_width: surface.border_width,
            border_color: surface.border_color.faded(opacity),
            border_radius: surface.radius,
        }
    }
}

/// Paints a generic box filling the rect.
pub fn paint(cr: &cairo::Context, rect: Rect, style: &Style) -> Result<()> {
    let (x, y, w, h) = (rect.x1, rect.y1, rect.w(), rect.h());
    let r = style.border_radius;

    // Path.
    cr.new_path();
    if r == 0. {
        cr.rectangle(x, y, w, h);
    } else {
        rounded_sub_path(cr, x, y, w, h, r);
    }

    // Background.
    let bg = style.bg;
    cr.set_source_rgba(bg.r, bg.g, bg.b, bg.a);
    cr.fill_preserve()
        .context("Could not fill shape on main surface")?;

    // Border.
    if style.border_width > 0. {
        cr.set_line_width(style.border_width);

        let color = style.border_color;
        cr.set_source_rgba(color.r, color.g, color.b, color.a);

        cr.stroke_preserve()
            .context("Could not stroke main surface path")?;
    }

    Ok(())
}
