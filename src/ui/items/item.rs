use std::{sync::Arc, time::Instant};

use anyhow::{Context, Result};

use crate::{
    config::ComputedConfig,
    notification::Notification,
    ui::renderables::{
        Rect,
        notification::{NotificationRenderable, Style, Transition},
    },
};

/// Represents a logical notification item.
#[derive(Debug)]
pub struct Item {
    config: Arc<ComputedConfig>,

    notification: Notification,
    dismissed: bool,

    style: Style,
    transitions: Vec<Transition>,

    notification_r: NotificationRenderable,
    bounds: Option<Rect>,
}

impl Item {
    pub fn new(config: Arc<ComputedConfig>, notification: Notification) -> Self {
        let notification_r = NotificationRenderable::new(&config, &notification);
        Self {
            config,

            notification,
            dismissed: false,

            style: Style::default(),
            transitions: vec![],

            notification_r,
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

    pub fn set_style(&mut self, style: Style) {
        self.style = style;
    }

    pub fn set_transitions(&mut self, transitions: Vec<Transition>) {
        self.transitions = transitions;
    }

    pub fn mark_dismissed(&mut self) {
        self.dismissed = true;
    }

    /// Progresses the transition attached to the item if any and a boolean indicating
    /// if all the transitions have settled.
    pub fn tick(&mut self, now: &Instant) -> bool {
        self.transitions.retain_mut(|transition| {
            let (style, settled) = transition.interpolate(&self.style, now);
            self.style = style;
            !settled
        });

        self.transitions.is_empty()
    }

    /// Renders the current item to a cairo canvas and returns its rect bounds.
    pub fn render(&mut self, cr: &cairo::Context) -> Result<Rect> {
        let bounds = self
            .notification_r
            .render(cr, &self.style)
            .context("Could not render notification")?;

        #[cfg(debug_assertions)]
        if self.config.debug_mode {
            cr.new_path();
            cr.set_source_rgba(0., 0., 255., 0.5);
            cr.rectangle(bounds.x1, bounds.y1, bounds.w(), bounds.h());
            let _ = cr.stroke();
        }

        self.bounds = Some(bounds);
        Ok(bounds)
    }

    pub fn content_size(&self) -> (f64, f64) {
        self.notification_r.content_size()
    }

    pub fn bounds(&self) -> Option<&Rect> {
        self.bounds.as_ref()
    }
}
