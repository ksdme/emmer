use anyhow::{Context, Result};
use debug_ignore::DebugIgnore;

use crate::dbus::notification::ImageSource;

/// A decoded image, scaled to fit, as premultiplied native endian ARGB32 pixels
/// with a stride of `width * 4`.
#[derive(Debug)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub pixels: DebugIgnore<Vec<u8>>,
}

/// Reads, decodes and scales the image down to fit within max_w and max_h. This is
/// expensive and blocking, so, it should be run off the UI thread.
pub fn decode(source: ImageSource, max_w: f64, max_h: Option<f64>) -> Result<Image> {
    let image = match source {
        ImageSource::File(path_buf) => image::ImageReader::open(&path_buf)
            .context("Could not read image file")?
            .with_guessed_format()
            .context("Could not guess format of the image")?
            .decode()
            .context("Could not decode image")?,
        ImageSource::Data(image) => {
            if image.has_alpha {
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
            }
        }
    };

    Ok(scale(image, max_w, max_h))
}

// Scales an image to fit and converts it into cairo's pixel format.
fn scale(image: image::DynamicImage, max_w: f64, max_h: Option<f64>) -> Image {
    let cur_w = image.width() as f64;
    let cur_h = image.height() as f64;

    let mut factor = max_w / cur_w;
    if let Some(max_h) = max_h {
        factor = factor.min(max_h / cur_h);
    }

    let scaled_image = image.resize(
        (cur_w * factor).round() as u32,
        (cur_h * factor).round() as u32,
        image::imageops::CatmullRom,
    );

    // The dimensions of the scaled image might not exactly match the requested
    // dimensions. While this might not affect drawing, we need to use correct size
    // for buffers.
    let width = scaled_image.width();
    let height = scaled_image.height();

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

    Image {
        width,
        height,
        pixels: DebugIgnore(pixels),
    }
}
