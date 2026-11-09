use anyhow::{Context, Result};
use smithay_client_toolkit::seat::pointer::{CursorIcon, PointerEvent};

use std::{
    collections::{BTreeMap, HashSet},
    rc::Rc,
    time::{Duration, Instant},
};

use crate::{
    config::ComputedConfig,
    dbus::{notification::Notification, service::CloseReason},
    ui::{
        anchors::{Anchor, VerticalAnchor},
        items::{Item, bar::Bar, layout::Layout},
        renderables::{Animated, Easing, Rect, Timing},
    },
};

/// The presentation of the current stack.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Presentation {
    Stacked,
    Spread,
}

#[derive(Debug)]
pub enum AppCommand {
    Redraw,
    SetCursor(CursorIcon),
    NotifyClosed(u32, CloseReason),
    NotifyAction(u32, String),
}

// The Time returned by our clock is an Instant. But, it might
// not match Instant::now(). So, this makes it explicit.
pub type Time = Instant;

// A Clock based on Instant that can be paused and resumed.
struct Clock {
    base: Instant,
    anchor: Option<Instant>,
}

impl Clock {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            base: now,
            anchor: Some(now),
        }
    }

    fn pause(&mut self) {
        self.base = self.now();
        self.anchor = None;
    }

    fn resume(&mut self) {
        if self.anchor.is_none() {
            self.anchor = Some(Instant::now());
        }
    }

    fn now(&self) -> Time {
        self.base
            + self
                .anchor
                .map_or(Duration::ZERO, |last| Instant::now() - last)
    }
}

/// The container for the items.
pub struct Stack {
    clock: Clock,

    config: Rc<ComputedConfig>,
    anchor: Anchor,
    presentation: Presentation,

    bar: Bar,
    items: BTreeMap<u32, Item>,

    // The distance by which the spread items are scrolled away from the far
    // edge, and the furthest they can be scrolled.
    scroll: Animated<f64>,
    overflow: f64,

    bounds: Option<Rect>,
    hovering: Option<u32>,
}

impl Stack {
    pub fn new(config: Rc<ComputedConfig>, anchor: Anchor) -> Self {
        Self {
            clock: Clock::new(),

            bar: Bar::new(config.clone()),
            items: BTreeMap::new(),

            config,
            anchor,
            presentation: Presentation::Stacked,

            scroll: Animated::new(0.),
            overflow: 0.,

            bounds: None,
            hovering: None,
        }
    }

    /// Update the anchor of the stack.
    pub fn set_anchor(&mut self, anchor: Anchor) {
        self.anchor = anchor;
        for item in self.items.values_mut() {
            item.set_anchor(&self.anchor);
        }
        self.update_visual_states();
    }

    // TODO: This needs to be as efficient as possible.
    // Finds an item that is at (x, y) position in the unscrolled layout.
    fn find_at_mut(&mut self, at: (f64, f64)) -> Option<&mut Item> {
        for el in self.items.values_mut().rev() {
            if let Some(hitbox) = el.target() {
                // Given that values are sorted, we can abort as soon as we
                // find one item that is definitely out of the pointer box.
                if match self.anchor.vertical {
                    VerticalAnchor::Top => hitbox.y1 > at.1,
                    VerticalAnchor::Bottom => hitbox.y2 < at.1,
                } {
                    break;
                }

                if hitbox.contains(at) {
                    return Some(el);
                }
            }
        }

        None
    }

    fn bar_hit(&self, at: (f64, f64)) -> bool {
        self.bar.target().is_some_and(|target| target.contains(at))
    }

    // The vertical translation that a scroll by distance applies to the items.
    fn scroll_offset(&self, distance: f64) -> f64 {
        match self.anchor.vertical {
            VerticalAnchor::Top => -distance,
            VerticalAnchor::Bottom => distance,
        }
    }

    // The region of the layer that the items scrolled by distance remain visible
    // in, which keeps them from running into the bar and the anchored margin.
    fn viewport(&self, distance: f64) -> Option<Rect> {
        if distance <= 0. {
            return None;
        }

        let origin = self.config.margin.y + self.bar.push();
        let w = self.anchor.layer_width;
        let h = (self.anchor.layer_height - origin).max(0.);
        Some(match self.anchor.vertical {
            VerticalAnchor::Top => Rect::from_xywh(0., origin, w, h),
            VerticalAnchor::Bottom => Rect::from_xywh(0., 0., w, h),
        })
    }

    // Maps a pointer event on the layer onto the unscrolled layout, if it lands
    // where the items are visible.
    fn content_event(&self, event: &PointerEvent) -> Option<PointerEvent> {
        let scroll = self.scroll.target();
        if self
            .viewport(scroll)
            .is_some_and(|v| !v.contains(event.position))
        {
            return None;
        }

        let (x, y) = event.position;
        Some(PointerEvent {
            position: (x, y - self.scroll_offset(scroll)),
            ..event.clone()
        })
    }

    // TODO: Lock the items.
    /// Pushes an item to the stack and returns a list of resulting side effects.
    pub fn push(&mut self, notification: Notification) -> Result<Vec<AppCommand>> {
        log::info!("stack.push: {:?}", notification.id);
        let item = Item::spawn(
            self.clock.now(),
            notification,
            self.config.clone(),
            &Self::layout(&self.config, &self.anchor, self.presentation, &self.bar),
        )
        .context("Could not spawn item")?;

        self.items.insert(item.id(), item);
        self.update_visual_states();

        Ok(vec![AppCommand::Redraw])
    }

    /// Removes expired items from the stack and returns a list of resulting
    /// side effects.
    pub fn dismiss_expired(&mut self) -> Vec<AppCommand> {
        let mut commands = vec![];

        let now = self.clock.now();
        for item in self.items.values_mut() {
            if item.is_expired(now) && item.dismiss() {
                commands.push(AppCommand::NotifyClosed(item.id(), CloseReason::Expired));
            }
        }

        if !commands.is_empty() {
            self.update_visual_states();
            commands.push(AppCommand::Redraw);
        }

        commands
    }

    /// Dismisses all the items and returns a list of resulting side effects.
    fn dismiss_all(&mut self) -> Vec<AppCommand> {
        let mut commands = self
            .items
            .values_mut()
            .filter_map(|item| {
                item.dismiss()
                    .then(|| AppCommand::NotifyClosed(item.id(), CloseReason::Manual))
            })
            .collect::<Vec<_>>();

        if !commands.is_empty() {
            self.update_visual_states();
            commands.push(AppCommand::Redraw);
        }

        commands
    }

    /// Updates the internal presentation flag and returns a boolean indicating
    /// if the change was accepted.
    fn set_presentation(&mut self, presentation: Presentation) -> bool {
        if self.presentation == presentation {
            return false;
        }

        self.presentation = presentation;

        // When the presentation is Spread, we assume that a user is interacting
        // with them. In that case, it doesn't make sense to dismiss them.
        // If there are some items that have almost expired, they might still be
        // dismissed though which is not ideal.
        match self.presentation {
            Presentation::Spread => self.clock.pause(),
            Presentation::Stacked => self.clock.resume(),
        }

        // Update the layout of the items.
        self.update_visual_states();

        true
    }

    // Takes only the fields it needs, so that the layout does not hold onto the
    // whole stack while the bar is being updated.
    fn layout<'a>(
        config: &'a ComputedConfig,
        anchor: &'a Anchor,
        presentation: Presentation,
        bar: &Bar,
    ) -> Layout<'a> {
        Layout {
            anchor,
            presentation,

            margin_x: config.margin.x,
            margin_y: config.margin.y,
            stack: &config.stack,
            spread: &config.spread,

            origin: config.margin.y + bar.push(),
        }
    }

    // Lays out the items and moves each of them towards their new target.
    fn update_visual_states(&mut self) {
        let now = Instant::now();

        let count = self
            .items
            .values()
            .filter(|item| !item.was_dismissed())
            .count();
        self.bar.set_count(count, now);

        let layout = Self::layout(&self.config, &self.anchor, self.presentation, &self.bar);
        self.bar.set_target(&layout, now);

        let targets = layout.targets(self.items.values().rev().map(Item::layout_input));
        for (item, target) in self.items.values_mut().rev().zip(targets) {
            item.set_target(target, now);
        }

        self.overflow = layout.overflow(self.items.values().rev().map(Item::layout_input));
        self.set_scroll(self.scroll.target().min(self.overflow), now);
    }

    fn set_scroll(&mut self, distance: f64, now: Instant) {
        self.scroll.animate_to(
            distance,
            Timing::new(Duration::from_millis(120), Easing::EaseOutCubic),
            now,
        );
    }

    /// The handler for when a pointer scrolls vertically by delta within the
    /// bounds of this stack.
    pub fn on_scroll(&mut self, event: &PointerEvent, delta: f64) -> Vec<AppCommand> {
        if self.presentation != Presentation::Spread {
            return vec![];
        }

        // The items move against the delta, so scrolling towards the far edge
        // reveals the items past it.
        let scroll = (self.scroll.target() - self.scroll_offset(delta)).clamp(0., self.overflow);
        if scroll == self.scroll.target() {
            return vec![];
        }
        self.set_scroll(scroll, Instant::now());

        // The items have moved under the pointer.
        let mut commands = self.on_hover(event);
        commands.push(AppCommand::Redraw);
        commands
    }

    /// The handler for when a pointer is hovering within the bounds of this
    /// stack.
    pub fn on_hover(&mut self, event: &PointerEvent) -> Vec<AppCommand> {
        // The bar is drawn over the items, so it takes precedence.
        if self.bar_hit(event.position) {
            let mut commands = self.bar.on_hover(event);

            if let Some(previous) = self.hovering.take()
                && let Some(previous) = self.items.get_mut(&previous)
            {
                commands.extend(previous.on_leave(event));
            }

            return commands;
        }
        let bar_commands = self.bar.on_leave();

        // Trigger on_hover on the current item.
        let content = self.content_event(event);
        let (hit, mut commands) = if let Some(content) = content.as_ref()
            && let Some(item) = self.find_at_mut(content.position)
        {
            let id = item.id();
            let mut commands = item.on_hover(content);

            if self.set_presentation(Presentation::Spread) {
                commands.push(AppCommand::Redraw);
            }

            (Some(id), commands)
        } else {
            (None, vec![AppCommand::SetCursor(CursorIcon::Default)])
        };

        // Allow the previously hovered item to reset itself.
        if hit != self.hovering
            && let Some(previous) = self.hovering
            && let Some(previous) = self.items.get_mut(&previous)
        {
            commands.extend(previous.on_leave(event));
        }
        self.hovering = hit;

        commands.extend(bar_commands);
        commands
    }

    /// The handler for when the pointer leaves the bounds of this stack.
    /// This method is only expected to be called once after a leave happens.
    pub fn on_leave(&mut self) -> Vec<AppCommand> {
        self.set_presentation(Presentation::Stacked);
        let _ = self.bar.on_leave();

        vec![
            AppCommand::Redraw,
            AppCommand::SetCursor(CursorIcon::Default),
        ]
    }

    /// The handler for when a pointer left click happens within the bounds of
    /// this stack.
    pub fn on_left_click(&mut self, event: &PointerEvent) -> Vec<AppCommand> {
        if self.bar_hit(event.position) {
            return if self.bar.on_left_click(event) {
                self.dismiss_all()
            } else {
                vec![]
            };
        }

        if let Some(content) = self.content_event(event)
            && let Some(item) = self.find_at_mut(content.position)
        {
            let (mut commands, update_visual_states) = item.on_left_click(&content);

            if update_visual_states {
                self.update_visual_states();
                commands.push(AppCommand::Redraw);
            }

            commands
        } else {
            vec![]
        }
    }

    /// The handler for when a pointer right click happens within the bounds of
    /// this stack.
    pub fn on_right_click(&mut self, event: &PointerEvent) -> Vec<AppCommand> {
        if self.bar_hit(event.position) {
            return vec![];
        }

        if let Some(content) = self.content_event(event)
            && let Some(item) = self.find_at_mut(content.position)
        {
            let (mut commands, update_visual_states) = item.on_right_click(&content);

            if update_visual_states {
                self.update_visual_states();
                commands.push(AppCommand::Redraw);
            }

            commands
        } else {
            vec![]
        }
    }

    // Renders the stack to the cairo canvas and returns a bool indicating if all the item
    // transitions have settled.
    pub fn render(&mut self, cx: &cairo::Context) -> Result<bool> {
        let now = Instant::now();

        let mut settled = self.scroll.tick(now);
        let mut full_bounds: Option<Rect> = None;
        let mut settled_dismissals = HashSet::<u32>::new();

        // The items are laid out unscrolled, so they are shifted as a whole and
        // clipped to the viewport while scrolled.
        let offset = self.scroll_offset(self.scroll.get());
        let viewport = self.viewport(self.scroll.get());

        cx.save().context("Could not save the context")?;
        if let Some(v) = viewport {
            cx.rectangle(v.x1, v.y1, v.w(), v.h());
            cx.clip();
        }
        cx.translate(0., offset);

        for (id, item) in self.items.iter_mut() {
            let item_settled = item.tick(now);

            // Render and update the scene bounds.
            if let Some(bounds) = item.render(cx).context("Could not render item: {id}")?
                && let bounds = bounds.translate(0., offset)
                && let Some(bounds) = viewport.map_or(Some(bounds), |v| bounds.intersection(&v))
            {
                full_bounds = Some(full_bounds.map_or(bounds, |fb| fb.union(&bounds)));
            }

            // If the item was marked as dismissed, and the transition
            // around it has settled, then, remove.
            if item_settled && item.was_dismissed() {
                settled_dismissals.insert(*id);
            }

            settled &= item_settled;
        }
        cx.restore().context("Could not restore the context")?;

        if !settled_dismissals.is_empty() {
            self.items.retain(|id, _| !settled_dismissals.contains(id));
        }

        // The bar is drawn last so that the items slide in and out from under it.
        settled &= self.bar.tick(now);
        if let Some(bounds) = self.bar.render(cx)? {
            full_bounds = Some(full_bounds.map_or(bounds, |fb| fb.union(&bounds)));
        }

        #[cfg(debug_assertions)]
        if self.config.debug_mode
            && let Some(b) = full_bounds
        {
            cx.new_path();
            cx.set_source_rgba(255., 0., 0., 0.5);
            cx.rectangle(b.x1, b.y1, b.w(), b.h());
            let _ = cx.stroke();
        }

        // While the stack is spread, the pointer is interacting with it, so its
        // bounds are not allowed to shrink vertically. Otherwise, the items
        // being dismissed could pull the input region out from under a pointer
        // that has not moved and end the interaction. Leaving horizontally
        // still ends it, after which the bounds can shrink.
        self.bounds = match (self.presentation, self.bounds, full_bounds) {
            (Presentation::Spread, Some(held), Some(bounds)) => Some(Rect {
                y1: held.y1.min(bounds.y1),
                y2: held.y2.max(bounds.y2),
                ..bounds
            }),
            _ => full_bounds,
        };

        Ok(settled)
    }

    pub fn bounds(&self) -> Option<&Rect> {
        self.bounds.as_ref()
    }
}
