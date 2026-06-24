use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};

use crate::{
    config::ComputedConfig,
    notification::{Action, Notification},
    ui::{
        items::stack::Presentation,
        renderables::{Rect, button, notification},
    },
};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum VisualState {
    Stacked { pos: usize, y: f64 },
    Spread { y: f64 },
    Hidden { y: f64 },
}

/// Represents an action button.
#[derive(Debug)]
pub struct ActionButton {
    action: Action,

    r: button::Renderable,
    style: button::Style,
    transition: Option<button::StyleTransition>,

    bounds: Option<Rect>,
}

/// Represents a logical notification item.
#[derive(Debug)]
pub struct Item {
    config: Arc<ComputedConfig>,

    visual_state: VisualState,
    dismissed: bool,

    notif: Notification,
    notif_r: notification::Renderable,
    notif_style: notification::Style,
    notif_transition: Option<notification::StyleTransition>,
    notif_bounds: Option<Rect>,

    action_buttons: Vec<(Action, ActionButton)>,
    buttons: bool,

    bounds: Option<Rect>,
}

impl Item {
    pub fn spawn(
        notif: Notification,
        config: Arc<ComputedConfig>,
        presentation: &Presentation,
    ) -> Self {
        let notif_r = notification::Renderable::new(&config, &notif);

        let (w, h) = notif_r.content_size();
        let notif_style = notification::Style {
            x: config.margin.x,
            y: match presentation {
                Presentation::Stack => -config.margin.y,
                Presentation::Spread => config.margin.y - config.spread.gap - h,
            },

            w,
            h,

            outer_opacity: 1.,
            inner_opacity: 1.,
        };

        Self {
            config,

            visual_state: VisualState::Hidden { y: 0. },
            dismissed: false,

            notif,
            notif_r,
            notif_style,
            notif_transition: None,
            notif_bounds: None,

            action_buttons: vec![],
            buttons: false,

            bounds: None,
        }
    }

    pub fn id(&self) -> u32 {
        self.notif.id()
    }

    pub fn notification(&self) -> &Notification {
        &self.notif
    }

    pub fn is_dimissed(&self) -> bool {
        self.dismissed
    }

    pub fn mark_dismissed(&mut self) {
        self.dismissed = true;
    }

    /// Updates the transitions and the style of the item as per the visual state.
    /// Returns the y position that the next visual element can start at if it does not
    /// want to overlap with the current item.
    // TODO: Transtions should not be applied if one towards the same target is underway.
    pub fn set_visual_state(&mut self, visual_state: VisualState, now: Instant) -> f64 {
        // The natural duration of a transition.
        let duration = Duration::from_millis(200);

        let (w, h) = self.notif_r.content_size();
        let (transition, y) = match visual_state {
            VisualState::Stacked { pos, y } if pos == 0 => {
                let target = notification::Style {
                    x: self.config.margin.x,
                    y,

                    w,
                    h,

                    outer_opacity: if self.dismissed { 0. } else { 1. },
                    inner_opacity: if self.dismissed { 0. } else { 1. },
                };

                (
                    notification::StyleTransition::new(duration, target.into(), Some(now)),
                    y + h,
                )
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

                (
                    notification::StyleTransition::new(duration, target, Some(now)),
                    y + self.config.stack.peek,
                )
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

                (
                    notification::StyleTransition::new(duration, target.into(), Some(now)),
                    y + h + self.config.spread.gap,
                )
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

                (
                    notification::StyleTransition::new(duration, target.into(), Some(now)),
                    y + self.config.stack.peek,
                )
            }
        };

        self.visual_state = visual_state;
        self.notif_transition = Some(transition);

        y
    }

    /// Progresses all the transition attached to the item and returns a boolean
    /// indicating if all the transitions have completed.
    pub fn tick(&mut self, now: &Instant) -> bool {
        // Progress the notification.
        let notif_complete = if let Some(notif_t) = self.notif_transition.as_mut() {
            let (style, complete) = notif_t.interpolate(&self.notif_style, now);

            self.notif_style = style;
            if complete {
                self.notif_transition = None;
            }

            complete
        } else {
            true
        };

        // Progress the action buttons.
        let mut buttons_complete = false;
        for button in self.action_buttons.iter_mut() {
            if let Some(button_t) = button.1.transition.as_mut() {
                let (style, complete) = button_t.interpolate(&button.1.style, now);

                button.1.style = style;
                if complete {
                    button.1.transition = None;
                }

                buttons_complete |= complete;
            }
        }

        notif_complete & buttons_complete
    }

    /// Renders the current item to a cairo canvas and returns its rect bounds.
    pub fn render(&mut self, cr: &cairo::Context) -> Result<Option<Rect>> {
        let notif_bounds = self
            .notif_r
            .render(cr, &self.notif_style)
            .context("Could not render notification")?;
        self.notif_bounds = Some(notif_bounds);

        #[cfg(debug_assertions)]
        if self.config.debug_mode {
            cr.new_path();
            cr.set_source_rgba(0., 0., 255., 0.5);
            cr.rectangle(
                notif_bounds.x1,
                notif_bounds.y1,
                notif_bounds.w(),
                notif_bounds.h(),
            );
            let _ = cr.stroke();
        }

        let mut bounds = notif_bounds;
        if self.buttons && self.action_buttons.len() > 0 {
            let gap = 8.;

            let mut x = notif_bounds.x1;
            let y = notif_bounds.y2 + gap;

            for button in self.action_buttons.iter_mut() {
                let button_bounds = button
                    .1
                    .r
                    .render(cr, &button.1.style, x, y)
                    .context("Could not draw")?;
                button.1.bounds = Some(button_bounds);

                x = button_bounds.x2 + gap;
            }

            bounds.y2 = y;
        }

        self.bounds = Some(bounds);
        Ok(Some(bounds))
    }

    pub fn bounds(&self) -> Option<&Rect> {
        self.bounds.as_ref()
    }
}
