use std::time::Instant;

use anyhow::{Context, Result};
use log::debug;
use skia_safe::{ImageInfo, surfaces};
use smithay_client_toolkit::shm::slot::SlotPool;
use wayland_client::{
    QueueHandle,
    protocol::{wl_shm::Format, wl_surface::WlSurface},
};

use crate::{
    renderer::complex::{
        self,
        block::{Block, Shadow},
    },
    ui::app::App,
};

struct VisualState {
    w: f32,
    h: f32,
    x: f32,
    y: f32,
    opacity: f32,
    text: bool,
}

struct TransitionState {
    duration: f32,
    started_at: Instant,
    target_state: VisualState,
}

macro_rules! interp {
    ($target:expr, $current:expr, $progress:expr) => {
        if $target != $current {
            $current + ($target - $current) * $progress
        } else {
            $current
        }
    };
}

impl TransitionState {
    pub fn tick(&self, current: &VisualState, now: Option<Instant>) -> (VisualState, bool) {
        // The progress of the transition.
        let progress = ((now.unwrap_or(Instant::now()) - self.started_at).as_millis() as f32
            / self.duration)
            .clamp(0., 1.);

        // Return an interpolated visual state along with a boolean indicating if the transition
        // is complete.
        let target = &self.target_state;
        (
            VisualState {
                w: interp!(target.w, current.w, progress),
                h: interp!(target.h, current.h, progress),
                x: interp!(target.x, current.x, progress),
                y: interp!(target.y, current.y, progress),
                opacity: interp!(target.opacity, current.opacity, progress),
                text: target.text,
            },
            progress >= 1.,
        )
    }
}

struct Item {
    content_h: f32,
    current_state: VisualState,
    transition_state: Option<TransitionState>,
}

impl Item {
    // Returns a boolean indicating if the transition has completed.
    pub fn draw_with_transition(&mut self, canvas: &skia_safe::Canvas) -> bool {
        let transition_complete = if let Some(transition) = &mut self.transition_state {
            let (visual_state, transition_complete) = transition.tick(&self.current_state, None);

            self.current_state = visual_state;
            if transition_complete {
                self.transition_state = None;
            }

            transition_complete
        } else {
            true
        };

        complex::block(
            canvas,
            &Block {
                shadow: Some(Shadow::SM),
                ..Default::default()
            },
            self.current_state.x,
            self.current_state.y,
            self.current_state.w,
            self.current_state.h,
            self.current_state.opacity,
        );

        transition_complete
    }
}

/// The state of the application and its associated drivers for the
/// event loop.
pub struct State {
    // TODO: For some reason, both cairo and smithay expect i32 for dimensions.
    pub width: i32,
    pub height: i32,
    items: Vec<Item>,
}

impl State {
    pub fn new() -> Self {
        Self {
            width: 0,
            height: 0,
            items: vec![],
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

        let mut request_callback = false;
        for item in self.items.iter_mut().rev() {
            request_callback |= !item.draw_with_transition(surface.canvas());
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
        if request_callback {
            debug!("requesting another frame");
            wl_surface.frame(qh, wl_surface.clone());
        }
        wl_surface.commit();

        Ok(())
    }

    pub fn add_item(&mut self) {
        if self.items.is_empty() {
            self.items.push(Item {
                content_h: 96.,
                current_state: VisualState {
                    w: self.width as f32 - 40.,
                    h: 96.,
                    x: 20.,
                    y: 20.,
                    opacity: 64.,
                    text: false,
                },
                transition_state: Some(TransitionState {
                    started_at: Instant::now(),
                    duration: 500.,
                    target_state: VisualState {
                        w: self.width as f32 - 40.,
                        h: 96.,
                        x: 20.,
                        y: 60.,
                        opacity: 255.,
                        text: false,
                    },
                }),
            });
        }
    }
}
