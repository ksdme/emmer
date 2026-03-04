use crate::renderer::{blur, color::Color, shapes};

/// Represents a border configuration.
pub struct Border {
    pub color: Color,
    pub width: f64,
}

/// Represents a drop shadow configuration.
pub struct Shadow {
    pub offset: (f64, f64),
    pub blur: u8,
    pub spread: f64,
    pub color: Color,
}

/// Represents a blank rounded card.
pub struct Card {
    pub bg: Color,
    pub border: Option<Border>,
    pub radius: Option<f64>,
    pub shadow: Option<Shadow>,
}

impl Card {
    pub fn draw(&self, cx: cairo::Context, x: f64, y: f64, w: f64, h: f64) {
        if let Some(shadow) = &self.shadow {
            let shadow_surface = {
                let mut off_surface = cairo::ImageSurface::create(
                    cairo::Format::ARgb32,
                    (w + 2.0 * shadow.spread) as i32,
                    (h + 2.0 * shadow.spread) as i32,
                )
                .expect("cairo offscreen");

                {
                    let off_cx = cairo::Context::new(&off_surface).expect("cairo context");
                    shapes::rect(&off_cx, shadow.spread, shadow.spread, w, h, self.radius);

                    // Fill the color.
                    let color = &shadow.color;
                    off_cx.set_source_rgba(color.r, color.g, color.b, color.a);
                    off_cx.fill().expect("cairo offscreen fill");
                }

                // Blur the surface.
                {
                    off_surface.flush();

                    let w = off_surface.width() as usize;
                    let h = off_surface.height() as usize;

                    let mut pixels = off_surface.data().expect("cairo offscreen pixels");
                    blur::stack_blur(&mut pixels, w, h, shadow.blur as usize);
                }

                off_surface
            };

            // Apply the shadow to the main canvas.
            cx.set_source_surface(
                &shadow_surface,
                x + shadow.offset.0 - shadow.spread,
                y + shadow.offset.1 - shadow.spread,
            )
            .expect("cairo offscreen surface");

            cx.paint().expect("cairo pain offscreen");
        }

        // Add base rectangle path.
        shapes::rect(&cx, x, y, w, h, self.radius);

        // Add background.
        let bg = &self.bg;
        cx.set_source_rgba(bg.r, bg.g, bg.b, bg.a);
        cx.fill_preserve().expect("cairo fill");

        // Add the border.
        if let Some(border) = &self.border {
            cx.set_line_width(border.width.into());

            let color = &border.color;
            cx.set_source_rgba(color.r, color.g, color.b, color.a);

            cx.stroke_preserve().expect("cairo stroke");
        }
    }
}
