use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use crate::{
    config::{ComputedConfig, Insets},
    notification::Action,
    ui::renderables::{Rect, button},
};

/// Represents an action button.
#[derive(Debug)]
pub struct ActionButton {
    action: Action,
    hovering: bool,

    r: button::Renderable,
    style: button::Style,
    transition: Option<button::StyleTransition>,

    bounds: Option<Rect>,
}

impl ActionButton {
    pub fn new(config: &ComputedConfig, action: Action) -> Self {
        Self {
            hovering: false,

            r: button::Renderable::new(config, action.label(), Insets { x: 16., y: 8. }),
            style: button::Style::default(),
            transition: None,

            action,

            bounds: None,
        }
    }

    pub fn on_hover(&mut self) {
        if self.hovering == false {
            self.hovering = true;

            // Lighten the button for feedback.
            self.transition = Some(button::StyleTransition::new(
                Duration::from_millis(100),
                button::PartialStyle {
                    light: Some(0.05),
                    ..Default::default()
                },
                None,
            ));
        }
    }

    pub fn on_leave(&mut self) {
        if self.hovering == true {
            self.hovering = false;

            // Reset the feedback on the button.
            self.transition = Some(button::StyleTransition::new(
                Duration::from_millis(100),
                button::PartialStyle {
                    light: Some(0.),
                    ..Default::default()
                },
                None,
            ));
        }
    }

    /// Progress the transition and clear it while returning a bool indicating if the
    /// transition is complete.
    pub fn tick(&mut self, now: &Instant) -> bool {
        if let Some(transition) = &mut self.transition {
            let (style, complete) = transition.interpolate(&self.style, now);

            self.style = style;
            if complete {
                self.transition = None;
            }

            complete
        } else {
            true
        }
    }

    pub fn render(&mut self, cr: &cairo::Context, x: f64, y: f64) -> Result<Rect> {
        let bounds = self
            .r
            .render(cr, &self.style, x, y)
            .context("Could not render button")?;

        self.bounds = Some(bounds);
        Ok(bounds)
    }

    pub fn bounds(&self) -> Option<&Rect> {
        self.bounds.as_ref()
    }
}
