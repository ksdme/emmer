use anyhow::{Context, Result};
use skia_safe::{ImageInfo, surfaces};
use smithay_client_toolkit::shm::slot::SlotPool;
use wayland_client::protocol::{wl_shm::Format, wl_surface::WlSurface};

use crate::renderer::complex::{
    self,
    block::{Block, Shadow},
};

/// The state of the application and its associated drivers for the
/// event loop.
pub struct State {
    // TODO: For some reason, both cairo and smithay expect i32 for dimensions.
    pub width: i32,
    pub height: i32,
}

impl State {
    pub fn new() -> Self {
        Self {
            width: 0,
            height: 0,
        }
    }

    pub fn draw(&self, wl_surface: &WlSurface, pool: &mut SlotPool) -> Result<()> {
        let mut surface = surfaces::raster_n32_premul((self.width, self.height))
            .context("Could not create skia surface")?;

        let indent = 12.;
        let peek = 10.;
        for no in 0..1 {
            let no = 2 - no;
            complex::block(
                surface.canvas(),
                &Block {
                    shadow: Some(Shadow::SM),
                    ..Default::default()
                },
                20. + indent * no as f32,
                20. + peek * no as f32,
                self.width as f32 - 40. - (no as f32 * 2. * indent),
                96.,
            );
        }

        let (frame_buffer, canvas) = pool
            .create_buffer(self.width, self.height, self.width * 4, Format::Argb8888)
            .context("Could not create buffer on pool")?;

        let image_info = ImageInfo::new_n32_premul((surface.width(), surface.height()), None);
        surface.read_pixels(
            &image_info,
            canvas,
            image_info.bytes_per_pixel() * image_info.width() as usize,
            (0, 0),
        );

        frame_buffer
            .attach_to(wl_surface)
            .context("Could not attach buffer")?;
        wl_surface.damage_buffer(0, 0, self.width, self.height);
        wl_surface.commit();

        Ok(())
    }
}
