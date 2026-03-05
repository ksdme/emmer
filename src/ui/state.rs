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
    // TODO: For some reason, both cairo and smithay expect i32 for dimensions.
    pub width: i32,
    pub height: i32,

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
        let mut cairo_surface = cairo::ImageSurface::create(
            cairo::Format::ARgb32,
            self.width as i32,
            self.height as i32,
        )
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
                        r: 0.85,
                        g: 0.85,
                        b: 0.85,
                    },
                    width: 1.0,
                }),
                radius: Some(6.0),
                shadow: Some(card::Shadow {
                    blur: 8.0,
                    offset: (0.0, 0.0),
                    color: Color {
                        a: 0.15,
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                    },
                }),
            };

            let padding = 32.0;
            let base_width = self.width as f64 - (2.0 * padding);
            let base_height = 96.0;
            let dedent = 16.0;
            let peek = 12.0;

            item.draw(
                &cx,
                padding + 2.0 * dedent,
                padding + 2.0 * peek,
                base_width - 4.0 * dedent,
                base_height,
            )
            .context("Could not draw card")?;

            item.draw(
                &cx,
                padding + 1.0 * dedent,
                padding + 1.0 * peek,
                base_width - 2.0 * dedent,
                base_height,
            )
            .context("Could not draw card")?;

            item.draw(&cx, padding, padding, base_width, base_height)
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
