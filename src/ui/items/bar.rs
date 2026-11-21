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
        renderables::{
            Animated, Easing, Rect, Renderable, Timing,
            bar::{self, Control, Mode},
        },
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
    mode: Mode,

    bar_r: bar::Bar,
    // The layout state of the bar, which is only ever driven by set_target.
    visual: AnimatedTarget,
    light: Animated<f64>,

    hovering: Option<Control>,
}

impl Bar {
    pub fn new(config: Rc<ComputedConfig>) -> Self {
        Self {
            bar_r: bar::Bar::new(config.clone(), 0, Mode::Expanded),
            config,

            count: 0,
            mode: Mode::Expanded,

            visual: AnimatedTarget::new(Target::default()),
            light: Animated::new(0.),
            hovering: None,
        }
    }

    fn shown(&self) -> bool {
        self.count > 1
    }

    /// Whether the notification cards behind the bar are hidden.
    pub fn stashed(&self) -> bool {
        self.mode == Mode::Stashed
    }

    /// The distance by which the bar pushes the items away from the anchored
    /// edge.
    pub fn push(&self) -> f64 {
        if self.shown() {
            self.bar_r.measure().h + self.config.theme.bar_gap
        } else {
            0.
        }
    }

    /// Updates the number of notifications the bar represents. This changes its
    /// push, so the layout should be refreshed with set_target afterwards.
    pub fn set_count(&mut self, count: usize, now: Instant) {
        // With only one card left there is nothing to stash, so reset before
        // rebuilding the label.
        if count <= 1 {
            self.mode = Mode::Expanded;
            let _ = self.set_hovering(None, now);
        }

        // The label is kept while hiding so that it does not change as it fades.
        if count > 1 && count != self.count {
            self.bar_r = bar::Bar::new(self.config.clone(), count, self.mode);
        }
        self.count = count;
    }

    /// Stashes or expands the cards behind the bar. Returns whether the state
    /// changed.
    pub fn set_stashed(&mut self, stashed: bool, now: Instant) -> bool {
        let mode = if stashed {
            Mode::Stashed
        } else {
            Mode::Expanded
        };
        if self.mode == mode || !self.shown() {
            return false;
        }

        self.mode = mode;
        self.bar_r = bar::Bar::new(self.config.clone(), self.count, mode);
        let _ = self.set_hovering(None, now);
        true
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

    fn control_at(&self, at: (f64, f64)) -> Option<Control> {
        self.target()
            .and_then(|rect| self.bar_r.control_at(rect, at))
    }

    // Updates the hover tint on the active control and returns a boolean
    // indicating if it changed.
    fn set_hovering(&mut self, hovering: Option<Control>, now: Instant) -> bool {
        if self.hovering == hovering {
            return false;
        }

        self.hovering = hovering;
        match hovering {
            Some(_) => self.light.animate_to(
                0.03,
                Timing::new(Duration::from_millis(100), Easing::EaseOutCubic),
                now,
            ),
            // Jump so we do not keep a lit_control-less fade that never paints.
            None => self.light.jump_to(0.),
        }

        true
    }

    pub fn on_hover(&mut self, event: &PointerEvent) -> Vec<AppCommand> {
        let hit = self.control_at(event.position);

        let mut commands = vec![AppCommand::SetCursor(if hit.is_some() {
            CursorIcon::Pointer
        } else {
            CursorIcon::Default
        })];

        if self.set_hovering(hit, Instant::now()) {
            commands.push(AppCommand::Redraw);
        }

        commands
    }

    pub fn on_leave(&mut self) -> Vec<AppCommand> {
        if self.set_hovering(None, Instant::now()) {
            vec![AppCommand::Redraw]
        } else {
            vec![]
        }
    }

    /// The left click handler on the bar. Returns the control that was clicked,
    /// if any.
    pub fn on_left_click(&mut self, event: &PointerEvent) -> Option<Control> {
        self.control_at(event.position)
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
                    control_light: self.light.get(),
                    lit_control: self.hovering,
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
