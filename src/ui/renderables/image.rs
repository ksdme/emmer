use std::io::Read;

use anyhow::{Context, Result};

use crate::ui::renderables::Rect;

/// Renders an image to the cairo surface with a specific width.
#[derive(Debug)]
pub struct Renderable {
    surface: cairo::ImageSurface,
}

impl Renderable {
    /// Returns a new prepared Image from a png source scaled to w width.
    pub fn from_png(mut png: impl Read, w: i32) -> Result<Self> {
        let source = cairo::ImageSurface::create_from_png(&mut png)
            .context("Could not create cairo base image surface")?;

        let (c_w, c_h) = (source.width() as f64, source.height() as f64);
        let scale_factor = w as f64 / c_w;

        let target =
            cairo::ImageSurface::create(cairo::Format::ARgb32, w, (scale_factor * c_h) as i32)
                .context("Could not create target surface")?;

        {
            let cr = cairo::Context::new(&target)
                .context("Could not create context for target surface")?;

            cr.scale(scale_factor, scale_factor);
            cr.set_source_surface(&source, 0., 0.)
                .context("Could not set source image on the context")?;

            cr.paint()
                .context("Could not paint image to target surface")?;
        }

        Ok(Self { surface: target })
    }

    pub fn content_size(&self) -> (f64, f64) {
        (self.surface.width() as f64, self.surface.height() as f64)
    }

    pub fn render(&self, cr: &cairo::Context, x: f64, y: f64, opacity: f64) -> Result<Rect> {
        cr.set_source_surface(&self.surface, x, y)
            .context("Could not set image")?;

        cr.paint_with_alpha(opacity)
            .context("Could not paint image")?;

        let (w, h) = self.content_size();
        Ok(Rect::from_xywh(x, y, w, h))
    }
}
