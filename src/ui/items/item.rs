use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};

use crate::{
    config::ComputedConfig,
    notification::Notification,
    ui::renderables::{Rect, notification},
};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum VisualState {
    Stacked { pos: usize, y: f64 },
    Spread { y: f64 },
    Hidden { y: f64 },
}

/// Represents a logical notification item.
#[derive(Debug)]
pub struct Item {
    config: Arc<ComputedConfig>,

    visual_state: VisualState,
    dismissed: bool,

    notification: Notification,
    notification_r: notification::Renderable,
    notification_s: notification::Style,
    notification_t: Vec<notification::StyleTransition>,

    bounds: Option<Rect>,
}

impl Item {
    pub fn new(
        config: Arc<ComputedConfig>,
        notification: Notification,
        visual_state: VisualState,
    ) -> Self {
        let notification_r = notification::Renderable::new(&config, &notification);
        Self {
            config,

            visual_state,
            dismissed: false,

            notification,
            notification_r,
            notification_s: notification::Style::default(),
            notification_t: vec![],

            bounds: None,
        }
    }

    pub fn id(&self) -> u32 {
        self.notification.id()
    }

    pub fn notification(&self) -> &Notification {
        &self.notification
    }

    pub fn is_dimissed(&self) -> bool {
        self.dismissed
    }

    pub fn mark_dismissed(&mut self) {
        self.dismissed = true;
    }

    pub fn set_style(&mut self, notification_s: notification::Style) {
        self.notification_s = notification_s;
    }

    /// Updates the transitions and the style of the item as per the visual state.
    /// Returns the y position that the next visual element can start at if it does not
    /// want to overlap with the current item.
    // TODO: Transtions should not be applied if one towards the same target is underway.
    pub fn set_visual_state(&mut self, visual_state: VisualState, now: Instant) -> f64 {
        // The natural duration of a transition.
        let duration = Duration::from_millis(200);

        let (w, h) = self.content_size();
        let (transitions, y) = match visual_state {
            VisualState::Stacked { pos, y } if pos == 0 => {
                let target = notification::Style {
                    x: self.config.margin.x,
                    y,

                    w,
                    h,

                    outer_opacity: if self.dismissed { 0. } else { 1. },
                    inner_opacity: if self.dismissed { 0. } else { 1. },
                };

                let transition =
                    notification::StyleTransition::new(duration, target.into(), Some(now));

                (vec![transition], y + h)
            }

            VisualState::Stacked { pos, y } => {
                let h = h.min(y - self.config.margin.y);
                let target = notification::PartialStyle {
                    x: Some(self.config.margin.x + (pos as f64) * self.config.stack.inset),
                    y: Some(y + self.config.stack.peek - h),

                    w: Some(self.config.width - 2. * (pos as f64) * self.config.stack.inset),
                    h: Some(h),

                    outer_opacity: Some(if self.dismissed { 0. } else { 1. }),
                    inner_opacity: Some(0.),
                };

                let transition = notification::StyleTransition::new(duration, target, Some(now));

                (vec![transition], y + self.config.stack.peek)
            }

            VisualState::Spread { y } => {
                let target = notification::Style {
                    x: self.config.margin.x,
                    y: if self.dismissed { y - h } else { y },

                    w,
                    h,

                    outer_opacity: if self.dismissed { 0. } else { 1. },
                    inner_opacity: if self.dismissed { 0. } else { 1. },
                };

                let transition =
                    notification::StyleTransition::new(duration, target.into(), Some(now));

                (vec![transition], y + h + self.config.spread.gap)
            }

            VisualState::Hidden { y } => {
                let target = notification::Style {
                    x: self.config.margin.x,
                    y: if self.dismissed { y - h } else { y },

                    w,
                    h,

                    outer_opacity: 0.,
                    inner_opacity: 0.,
                };

                let transition =
                    notification::StyleTransition::new(duration, target.into(), Some(now));

                (vec![transition], y + self.config.stack.peek)
            }
        };

        self.visual_state = visual_state;
        self.notification_t = transitions;

        y
    }

    /// Progresses the transition attached to the item if any and a boolean indicating
    /// if all the transitions have settled.
    pub fn tick(&mut self, now: &Instant) -> bool {
        self.notification_t.retain_mut(|transition| {
            let (style, settled) = transition.interpolate(&self.notification_s, now);
            self.notification_s = style;
            !settled
        });

        self.notification_t.is_empty()
    }

    /// Renders the current item to a cairo canvas and returns its rect bounds.
    pub fn render(&mut self, cr: &cairo::Context) -> Result<Option<Rect>> {
        // If the item is not visible, skip putting it on the canvas.
        if self.notification_s.outer_opacity == 0. {
            return Ok(None);
        }

        let bounds = self
            .notification_r
            .render(cr, &self.notification_s)
            .context("Could not render notification")?;

        #[cfg(debug_assertions)]
        if self.config.debug_mode {
            cr.new_path();
            cr.set_source_rgba(0., 0., 255., 0.5);
            cr.rectangle(bounds.x1, bounds.y1, bounds.w(), bounds.h());
            let _ = cr.stroke();
        }

        self.bounds = Some(bounds);
        Ok(Some(bounds))
    }

    pub fn content_size(&self) -> (f64, f64) {
        self.notification_r.content_size()
    }

    pub fn bounds(&self) -> Option<&Rect> {
        self.bounds.as_ref()
    }
}
