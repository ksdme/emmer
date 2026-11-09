use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use smithay_client_toolkit::seat::pointer::{CursorIcon, PointerEvent};

use crate::{
    config::ComputedConfig,
    transitionable,
    ui::{
        items::{layout::Layout, stack::AppCommand},
        renderables::{Animated, Easing, Rect, Renderable, Timing, bar},
    },
};

transitionable!(
    /// Where and how the bar should end up.
    Target {
        rect: Rect,
        opacity: f64,
    }
);

/// Represents the dismiss-all bar that sits on the anchored edge of the stack
/// while there is more than one notification.
#[derive(Debug)]
pub struct Bar {
    config: Rc<ComputedConfig>,

    count: usize,

    bar_r: bar::Bar,
    // The layout state of the bar, which is only ever driven by set_target.
    visual: AnimatedTarget,
    // The hover tint on the clear control.
    light: Animated<f64>,

    hovering_clear: bool,
}

impl Bar {
    pub fn new(config: Rc<ComputedConfig>) -> Self {
        Self {
            bar_r: bar::Bar::new(&config, 0),
            config,

            count: 0,

            visual: AnimatedTarget::new(Target::default()),
            light: Animated::new(0.),
            hovering_clear: false,
        }
    }

    fn shown(&self) -> bool {
        self.count > 1
    }

    /// The distance by which the bar pushes the items away from the anchored
    /// edge.
    pub fn push(&self) -> f64 {
        if self.shown() {
            self.bar_r.measure().h + self.config.spread.gap
        } else {
            0.
        }
    }

    /// Updates the number of notifications the bar represents. This changes its
    /// push, so the layout should be refreshed with set_target afterwards.
    pub fn set_count(&mut self, count: usize, now: Instant) {
        // The label is kept while hiding so that it does not change as it fades.
        if count > 1 && count != self.count {
            self.bar_r = bar::Bar::new(&self.config, count);
        }
        self.count = count;

        if !self.shown() {
            let _ = self.set_hovering_clear(false, now);
        }
    }

    /// Moves the bar towards its target in the layout. The bar always sits
    /// just outside the stack, so once its push is taken away, it slides out
    /// past the anchored edge.
    pub fn set_target(&mut self, layout: &Layout, now: Instant) {
        let size = self.bar_r.measure();

        // A fully hidden bar always appears from where it would be without its push.
        if self.visual.opacity.get() <= 0. && self.visual.opacity.target() <= 0. {
            let hidden = Layout {
                origin: layout.origin - self.push(),
                ..*layout
            };
            self.visual = AnimatedTarget::new(Target {
                rect: hidden.spawn(size),
                opacity: 0.,
            });
        }

        self.visual.animate_to(
            Target {
                rect: layout.spawn(size),
                opacity: if self.shown() { 1. } else { 0. },
            },
            Timing::new(Duration::from_millis(200), Easing::EaseOutCubic),
            now,
        );
    }

    /// The rect the bar occupies in the current layout, if it is interactable.
    pub fn target(&self) -> Option<Rect> {
        let target = self.visual.target();
        (target.opacity > 0.).then_some(target.rect)
    }

    fn clear_hit(&self, at: (f64, f64)) -> bool {
        self.target()
            .is_some_and(|rect| self.bar_r.clear_rect(rect).contains(at))
    }

    // Updates the hover tint on the clear control and returns a boolean
    // indicating if it changed.
    fn set_hovering_clear(&mut self, hovering: bool, now: Instant) -> bool {
        if self.hovering_clear == hovering {
            return false;
        }

        self.hovering_clear = hovering;
        self.light.animate_to(
            if hovering { 0.03 } else { 0. },
            Timing::new(Duration::from_millis(100), Easing::EaseOutCubic),
            now,
        );

        true
    }

    pub fn on_hover(&mut self, event: &PointerEvent) -> Vec<AppCommand> {
        let hit = self.clear_hit(event.position);

        let mut commands = vec![AppCommand::SetCursor(if hit {
            CursorIcon::Pointer
        } else {
            CursorIcon::Default
        })];

        if self.set_hovering_clear(hit, Instant::now()) {
            commands.push(AppCommand::Redraw);
        }

        commands
    }

    pub fn on_leave(&mut self) -> Vec<AppCommand> {
        if self.set_hovering_clear(false, Instant::now()) {
            vec![AppCommand::Redraw]
        } else {
            vec![]
        }
    }

    /// The left click handler on the bar. Returns a boolean indicating if all
    /// the notifications should be dismissed.
    pub fn on_left_click(&mut self, event: &PointerEvent) -> bool {
        self.clear_hit(event.position)
    }

    /// Progresses all the transitions attached to the bar and returns a boolean
    /// indicating if all of them have completed.
    pub fn tick(&mut self, now: Instant) -> bool {
        self.visual.tick(now) & self.light.tick(now)
    }

    /// Renders the bar to a cairo canvas and returns the rect it was drawn in.
    pub fn render(&self, cr: &cairo::Context) -> Result<Option<Rect>> {
        let visual = self.visual.get();

        if visual.opacity <= 0. {
            return Ok(None);
        }

        let rect = visual.rect;
        self.bar_r
            .paint(
                cr,
                rect,
                &bar::Style {
                    clear_light: self.light.get(),
                    opacity: visual.opacity,
                },
            )
            .context("Could not render bar")?;

        #[cfg(debug_assertions)]
        if self.config.debug_mode {
            cr.new_path();
            cr.set_source_rgba(0., 0., 255., 0.5);
            cr.rectangle(rect.x1, rect.y1, rect.w(), rect.h());
            let _ = cr.stroke();
        }

        Ok(Some(rect))
    }
}
