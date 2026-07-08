use anyhow::{Context, Result};

use crate::{
    dbus::notification,
    ui::renderables::{Rect, rounded_sub_path},
};

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
                let image = image::ImageReader::open(&path_buf)
                    .context("Could not read image file")?
                    .with_guessed_format()
                    .context("Could not guess format of the image")?
                    .decode()
                    .context("Could not decode image")?;

                Ok(Self {
                    surface: scaled_image_surface(image, w)
                        .context("Could not scale, create surface from image")?,
                })
            }
            notification::ImageSource::Data(image) => {
                let image = if image.has_alpha {
                    image::DynamicImage::ImageRgba8(
                        image::RgbaImage::from_raw(
                            image.width as u32,
                            image.height as u32,
                            image.data.0,
                        )
                        .context("Could not create image")?,
                    )
                } else {
                    image::DynamicImage::ImageRgb8(
                        image::RgbImage::from_raw(
                            image.width as u32,
                            image.height as u32,
                            image.data.0,
                        )
                        .context("Could not create image")?,
                    )
                };

                Ok(Self {
                    surface: scaled_image_surface(image, w)
                        .context("Could not scale, create surface from image")?,
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

// Scales an image to w and returns a cairo surface from it.
fn scaled_image_surface(image: image::DynamicImage, w: i32) -> Result<cairo::ImageSurface> {
    let scaled_image = image.resize(
        w as u32,
        ((w as f64 / image.width() as f64) * image.height() as f64) as u32,
        image::imageops::CatmullRom,
    );

    // The dimensions of the scaled image might not exactly match the requested
    // dimensions. While this might not affect drawing, we need to use correct size
    // for buffers.
    let scaled_w = scaled_image.width();
    let scaled_h = scaled_image.height();

    // Cairo always uses 32 bits to represent a pixel on the surface. So, it is simpler
    // to just treat all images as having alpha.
    let mut pixels = scaled_image.into_rgba8().into_raw();

    // CAIRO_FORMAT_ARGB32: This format uses 8 bits each for Alpha, Red, Green, and Blue.
    // It uses pre-multiplied alpha, meaning color channels are already multiplied by the
    // alpha value (e.g., 50% transparent red is 0x80800000, not 0x80ff0000)
    for px in pixels.chunks_exact_mut(4) {
        let r = px[0] as u16;
        let g = px[1] as u16;
        let b = px[2] as u16;
        let a = px[3] as u16;

        // Also round up.
        let r = ((r * a + 127) / 255) as u8;
        let g = ((g * a + 127) / 255) as u8;
        let b = ((b * a + 127) / 255) as u8;
        let a = a as u8;

        #[cfg(target_endian = "little")]
        {
            px[0] = b;
            px[1] = g;
            px[2] = r;
            px[3] = a;
        }

        #[cfg(target_endian = "big")]
        {
            px[0] = a;
            px[1] = r;
            px[2] = g;
            px[3] = b;
        }
    }

    // Transfer the image::Image to a cairo canvas.
    let source = cairo::ImageSurface::create_for_data(
        pixels,
        cairo::Format::ARgb32,
        scaled_w as i32,
        scaled_h as i32,
        scaled_w as i32 * 4,
    )
    .context("Could not create source surface")?;

    // Clip the image to a rounded rect using a duplicate cairo surface.
    // We could do this in place using Operator::Clear or itering through pixels.
    let target =
        cairo::ImageSurface::create(cairo::Format::ARgb32, scaled_w as i32, scaled_h as i32)
            .context("Could not create target")?;

    {
        let cr = cairo::Context::new(&target).context("Could not create context")?;

        rounded_sub_path(
            &cr,
            0.,
            0.,
            scaled_w as f64,
            scaled_h as f64,
            // Otherwise, narrow images have awkward full rounded corners.
            (scaled_h as f64 / 3.).min(6.),
        );
        cr.clip();

        cr.set_source_surface(source, 0., 0.)
            .context("Could not set source")?;

        cr.paint().context("Could not paint")?;
    }

    Ok(target)
}
