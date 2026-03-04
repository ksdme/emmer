use anyhow::{Context, Result};
use smithay_client_toolkit::shm::slot::SlotPool;
use wayland_client::protocol::{wl_shm::Format, wl_surface::WlSurface};

use crate::renderer::{
    color::Color,
    complex::{self, card},
};

/// The state of the application and its associated drivers for the
/// event loop.
pub struct State {
    width: u16,
    height: u16,

    done: bool,
}

impl State {
    pub fn new() -> Self {
        Self {
            width: 0,
            height: 0,

            done: false,
        }
    }

    pub fn draw(&self, surface: &WlSurface, pool: &mut SlotPool) -> Result<()> {
        let mut cairo_surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 256, 256)
            .context("Could not create cairo surface")?;

        {
            let cx = cairo::Context::new(&cairo_surface)
                .context("Could not create cairo context on frame surface")?;

            let item = card::Card {
                bg: Color {
                    a: 1.0,
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                },
                border: Some(card::Border {
                    color: Color {
                        a: 1.0,
                        r: 0.75,
                        g: 0.75,
                        b: 0.75,
                    },
                    width: 1.0,
                }),
                radius: Some(3.0),
                shadow: Some(card::Shadow {
                    blur: 8.0,
                    offset: (0.0, 0.0),
                    color: Color {
                        a: 0.25,
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                    },
                }),
            };

            item.draw(&cx, 32.0, 36.0, 184.0, 80.0)
                .context("Could not draw card")?;

            item.draw(&cx, 24.0, 28.0, 200.0, 80.0)
                .context("Could not draw card")?;

            item.draw(&cx, 16.0, 20.0, 224.0, 80.0)
                .context("Could not draw card")?;
        }

        let pixels = cairo_surface
            .data()
            .context("Could not get data on frame surface")?;

        let (frame_buffer, canvas) = pool
            .create_buffer(256, 256, 256 * 4, Format::Argb8888)
            .context("Could not create buffer on pool")?;

        canvas.copy_from_slice(&pixels);

        frame_buffer
            .attach_to(surface)
            .context("Could not attach buffer")?;
        surface.damage_buffer(0, 0, 256, 256);
        surface.commit();

        Ok(())
    }
}
