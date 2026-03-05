use anyhow::{Context, Result};
use smithay_client_toolkit::shm::slot::SlotPool;
use wayland_client::protocol::{wl_shm::Format, wl_surface::WlSurface};

use crate::renderer::complex::block::Block;

struct RenderState {
    w: f64,
    h: f64,
    x: f64,
    y: f64,
    opacity: f64,
}

struct Card {
    target_state: Option<RenderState>,
    current_state: Option<RenderState>,
}

/// The state of the application and its associated drivers for the
/// event loop.
pub struct State {
    // TODO: For some reason, both cairo and smithay expect i32 for dimensions.
    pub width: i32,
    pub height: i32,

    cards: Vec<Card>,
}

impl State {
    pub fn new() -> Self {
        Self {
            width: 0,
            height: 0,
            cards: vec![],
        }
    }

    pub fn draw(&self, surface: &WlSurface, pool: &mut SlotPool) -> Result<()> {
        let mut cairo_surface =
            cairo::ImageSurface::create(cairo::Format::ARgb32, self.width, self.height)
                .context("Could not create cairo surface")?;

        let padding = 16.0;
        {
            let cx = cairo::Context::new(&cairo_surface)
                .context("Could not create cairo context on frame surface")?;

            Block::default()
                .draw(
                    &cx,
                    padding,
                    padding,
                    self.width as f64 - (2.0 * padding),
                    92.0,
                )
                .context("Could not draw card")?;
        }

        let pixels = cairo_surface
            .data()
            .context("Could not get data on frame surface")?;

        let (frame_buffer, canvas) = pool
            .create_buffer(self.width, self.height, self.width * 4, Format::Argb8888)
            .context("Could not create buffer on pool")?;

        canvas.copy_from_slice(&pixels);

        frame_buffer
            .attach_to(surface)
            .context("Could not attach buffer")?;
        surface.damage_buffer(0, 0, self.width, self.height);
        surface.commit();

        Ok(())
    }
}
