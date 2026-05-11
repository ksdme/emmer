use anyhow::{Context, Result};
use log::trace;
use skia_safe::{ImageInfo, surfaces};
use smithay_client_toolkit::shm::slot::SlotPool;
use wayland_client::{
    QueueHandle,
    protocol::{wl_shm::Format, wl_surface::WlSurface},
};

use crate::{
    config::{Config, Theme},
    engine::items::{LayoutMode, Stack},
    ui::app::App,
};

/// The state of the application and its associated drivers for the
/// event loop.
pub struct State {
    // TODO: For some reason, both cairo and smithay expect i32 for dimensions.
    pub width: i32,
    pub height: i32,

    pub stack: Stack,

    config: Config,
    theme: Theme,
}

impl State {
    pub fn new(config: Config) -> Self {
        Self {
            width: 0,
            height: 0,

            stack: Stack::new(),

            theme: Theme::from(&config.theme),
            config,
        }
    }

    pub fn draw(
        &mut self,
        qh: &QueueHandle<App>,
        wl_surface: &WlSurface,
        pool: &mut SlotPool,
    ) -> Result<()> {
        let mut surface = surfaces::raster_n32_premul((self.width, self.height))
            .context("Could not create skia surface")?;

        let request_callback = self.stack.draw(&self.theme, surface.canvas());
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
        if request_callback {
            trace!(target: "draw", "requesting another frame");
            wl_surface.frame(qh, wl_surface.clone());
        }
        wl_surface.commit();

        Ok(())
    }

    pub fn push(&mut self) {
        self.stack.push(&self.config);
    }

    pub fn dismiss(&mut self, at: (f32, f32)) {
        self.stack.dismiss(&self.config, at);
    }

    pub fn set_mode(&mut self, mode: LayoutMode) {
        self.stack.set_mode(&self.config, mode);
    }
}
