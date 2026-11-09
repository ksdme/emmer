use anyhow::{Context, Result};

use crate::{
    dbus,
    ui::renderables::{Rect, Renderable, Size, rounded_sub_path},
};

/// Renders an image to the cairo surface with a specific width.
#[derive(Debug)]
pub struct Image {
    surface: cairo::ImageSurface,
}

impl Image {
    /// Builds the instance of the renderable from the decoded dbus image with
    /// its corners rounded by radius.
    pub fn new(image: dbus::image::Image, radius: f64) -> Result<Self> {
        Ok(Self {
            surface: rounded_surface(image, radius)
                .context("Could not create surface from image")?,
        })
    }
}

impl Renderable for Image {
    type Style = Style;

    fn measure(&self) -> Size {
        Size {
            w: self.surface.width() as f64,
            h: self.surface.height() as f64,
        }
    }

    fn paint(&self, cr: &cairo::Context, rect: Rect, style: &Style) -> Result<()> {
        cr.set_source_surface(&self.surface, rect.x1, rect.y1)
            .context("Could not set image")?;

        cr.paint_with_alpha(style.opacity)
            .context("Could not paint image")?;

        Ok(())
    }
}

/// The style of an image renderable.
#[derive(Debug, Default, Clone, Copy)]
pub struct Style {
    pub opacity: f64,
}

// Creates a cairo surface from the image with its corners rounded.
fn rounded_surface(image: dbus::image::Image, radius: f64) -> Result<cairo::ImageSurface> {
    let scaled_w = image.width;
    let scaled_h = image.height;

    // Transfer the pixels to a cairo canvas.
    let source = cairo::ImageSurface::create_for_data(
        image.pixels.0,
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
            (scaled_h as f64 / 3.).min(radius),
        );
        cr.clip();

        cr.set_source_surface(source, 0., 0.)
            .context("Could not set source")?;

        cr.paint().context("Could not paint")?;
    }

    Ok(target)
}
