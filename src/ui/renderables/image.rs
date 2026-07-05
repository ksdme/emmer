use std::fs::File;

use anyhow::{Context, Result};

use crate::{dbus::notification, ui::renderables::Rect};

/// Renders an image to the cairo surface with a specific width.
#[derive(Debug)]
pub struct Renderable {
    surface: cairo::ImageSurface,
}

impl Renderable {
    /// Builds the instance of the renderable from the dbus image source.
    pub fn from_source(source: notification::ImageSource, w: i32) -> Result<Self> {
        match source {
            notification::ImageSource::File(path_buf) => {
                let mut file = File::open(&path_buf).context("Could not read file")?;

                let source = cairo::ImageSurface::create_from_png(&mut file)
                    .context("Could not create cairo base image surface")?;

                Ok(Self {
                    surface: scale_surface(source, w).context("Could not scale surface")?,
                })
            }
            notification::ImageSource::Data(image) => {
                let source = cairo::ImageSurface::create_for_data(
                    image.data.0,
                    if image.channels == 4 {
                        cairo::Format::ARgb32
                    } else {
                        cairo::Format::Rgb24
                    },
                    image.width,
                    image.height,
                    image.row_stride,
                )
                .context("Could not create source surface")?;

                Ok(Self {
                    surface: scale_surface(source, w).context("Could not scale surface")?,
                })
            }
        }
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

fn scale_surface(source: cairo::ImageSurface, w: i32) -> Result<cairo::ImageSurface> {
    let (c_w, c_h) = (source.width() as f64, source.height() as f64);
    let scale_factor = w as f64 / c_w;

    let target = cairo::ImageSurface::create(cairo::Format::ARgb32, w, (scale_factor * c_h) as i32)
        .context("Could not create target surface")?;

    {
        let cr =
            cairo::Context::new(&target).context("Could not create context for target surface")?;

        // Implicit quality of scaling is "best", otherwise we will have to create
        // a pattern object and set the quality on it.
        cr.scale(scale_factor, scale_factor);

        cr.set_source_surface(&source, 0., 0.)
            .context("Could not set source image on the context")?;

        cr.paint()
            .context("Could not paint image to target surface")?;
    }

    Ok(target)
}
