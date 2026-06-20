use anyhow::{Context, Result};
use smithay_client_toolkit::seat::pointer::{CursorIcon, PointerEvent};

use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    config::ComputedConfig,
    dbus::CloseReason,
    notification::Notification,
    ui::{
        items::Item,
        renderables::{
            Rect,
            notification::{PartialStyle, Style, Transition},
        },
    },
};

#[derive(Debug)]
pub enum AppCommand {
    Redraw,
    SetCursor(CursorIcon),
    NotifyClosed(u32, CloseReason),
}

/// Represents the presentation mode of the stack.
#[derive(Debug, Clone, PartialEq)]
pub enum Presentation {
    Stack,
    List,
}

/// The container for the items.
pub struct Stack {
    config: Arc<ComputedConfig>,

    items: BTreeMap<u32, Item>,
    presentation: Presentation,

    bounds: Option<Rect>,
}

impl Stack {
    pub fn new(config: Arc<ComputedConfig>) -> Self {
        Self {
            config,

            items: BTreeMap::new(),
            presentation: Presentation::Stack,

            bounds: None,
        }
    }

    /// Finds an item that is at (x, y) visual position on this stack.
    pub fn find_at(&self, at: (f64, f64)) -> Option<&Item> {
        for el in self.items.values().rev() {
            if let Some(hitbox) = el.bounds() {
                // TODO: Requires fixes when allowed alt anchors.
                // Given that values are sorted, we can abort as soon as we
                // find one item that is definitely out of the pointer box.
                if hitbox.y1 > at.1 {
                    break;
                }

                if hitbox.contains(at) {
                    return Some(el);
                }
            }
        }

        None
    }

    // TODO: Lock the items.
    /// Pushes an item to the stack and returns a list of resulting side effects.
    pub fn push(&mut self, notification: Notification) -> Vec<AppCommand> {
        log::info!("stack.push: {:?}", notification.id());
        let mut item = Item::new(&self.config, notification);

        let (w, h) = item.content_size();
        item.set_style(Style {
            x: self.config.margin.x,
            y: match self.presentation {
                Presentation::List => self.config.margin.y - self.config.spread.gap - h,
                Presentation::Stack => -self.config.margin.y,
            },

            w,
            h,

            outer_opacity: 1.,
            inner_opacity: 1.,
        });

        self.items.insert(item.id(), item);
        self.recompute_layout();

        vec![AppCommand::Redraw]
    }

    // TODO: Lock the items.
    /// Removes an item from the stack and returns a list of resulting
    /// side effects.
    fn dismiss(&mut self, id: u32) -> Vec<AppCommand> {
        if let Some(item) = self.items.get_mut(&id)
            && !item.is_dimissed()
        {
            item.mark_dismissed();
            self.recompute_layout();

            vec![
                AppCommand::Redraw,
                AppCommand::NotifyClosed(id, CloseReason::Manual),
            ]
        } else {
            vec![]
        }
    }

    /// Removes expired items from the stack and returns a list of resulting
    /// side effects.
    pub fn dismiss_expired(&mut self) -> Vec<AppCommand> {
        let mut commands = vec![];

        for item in self.items.values_mut() {
            if item.notification().is_expired() && !item.is_dimissed() {
                item.mark_dismissed();
                commands.push(AppCommand::NotifyClosed(item.id(), CloseReason::Expired));
            }
        }

        if !commands.is_empty() {
            self.recompute_layout();
            commands.push(AppCommand::Redraw);
        }

        commands
    }

    /// Updates the internal presentation flag and returns a boolean indicating
    /// if the change was accepted.
    fn set_presentation(&mut self, presentation: Presentation) -> bool {
        if self.presentation != presentation {
            log::info!("stack.set_presentation: {:?}", presentation);

            self.presentation = presentation;
            self.recompute_layout();

            true
        } else {
            false
        }
    }

    fn recompute_layout(&mut self) {
        let now = Instant::now();
        match self.presentation {
            Presentation::List => self.recompute_layout_list(now),
            Presentation::Stack => self.recompute_layout_stack(now),
        }
    }

    fn recompute_layout_list(&mut self, now: Instant) {
        let mut no = 0;
        let mut top_y = self.config.margin.y;

        for (_, item) in self.items.iter_mut().rev() {
            let (item_w, item_h) = item.content_size();

            // Show the first config.spread.max_count items.
            if no <= self.config.spread.max_count {
                let target = Style {
                    x: self.config.margin.x,
                    y: if item.is_dimissed() {
                        top_y - item_h
                    } else {
                        top_y
                    },

                    w: item_w,
                    h: item_h,

                    outer_opacity: if item.is_dimissed() { 0. } else { 1. },
                    inner_opacity: if item.is_dimissed() { 0. } else { 1. },
                };

                // If the item is dismissed we want the content to be overlaid
                // on top of this one, so, we don't increase offset.
                if !item.is_dimissed() {
                    no += 1;
                    top_y = top_y + target.h + self.config.spread.gap;
                }

                item.set_transitions(vec![Transition::new(
                    Duration::from_millis(200),
                    target.into(),
                    Some(now),
                )]);
            } else {
                // The rest of the items should naturally just go sit at the bottom.
                // It doesn't matter if all the other items sit are on top of each other
                // because they won't be visible.
                let target = Style {
                    x: self.config.margin.x,
                    y: top_y + self.config.spread.gap,

                    w: item_w,
                    h: item_h,

                    outer_opacity: 0.,
                    inner_opacity: 0.,
                };

                item.set_transitions(
                    // We are using a transition here instead of setting the value
                    // immediately so a new item will also act as expected.
                    vec![Transition::new(
                        Duration::from_millis(200),
                        target.into(),
                        Some(now),
                    )],
                );
            }
        }
    }

    fn recompute_layout_stack(&mut self, now: Instant) {
        let stack_max_count = self.config.stack.max_count as f64;

        let mut no = 0.;
        let mut top_y = self.config.margin.y;

        for (_, item) in self.items.iter_mut().rev() {
            let (item_w, item_h) = item.content_size();

            // Renders the first item as a regular block.
            if no == 0. {
                let target = Style {
                    x: self.config.margin.x,
                    y: top_y,

                    w: item_w,
                    h: item_h,

                    outer_opacity: if item.is_dimissed() { 0. } else { 1. },
                    inner_opacity: if item.is_dimissed() { 0. } else { 1. },
                };

                // If the item is dismissed we want the content to be overlaid
                // on top of this one, so, we don't increase offset.
                if !item.is_dimissed() {
                    no += 1.;
                    top_y = target.y + target.h;
                }

                item.set_transitions(vec![Transition::new(
                    Duration::from_millis(200),
                    target.into(),
                    Some(now),
                )]);
            } else if no < stack_max_count {
                // Render the stack entries.

                // The height of the card should be smaller than the top-most card.
                let h = item_h.min(top_y - self.config.margin.y);
                let target = PartialStyle {
                    x: Some(self.config.margin.x + no * self.config.stack.inset),
                    y: Some(top_y + self.config.stack.peek - h),

                    w: Some(self.config.width - 2. * no * self.config.stack.inset),
                    h: Some(h),

                    outer_opacity: Some(if item.is_dimissed() { 0. } else { 1. }),
                    inner_opacity: Some(0.),
                };

                // If the item is dismissed we want the content to be overlaid
                // on top of this one, so, we don't increase offset.
                if !item.is_dimissed() {
                    no += 1.;
                    top_y = target.y.unwrap_or_default() + target.h.unwrap_or_default();
                }

                item.set_transitions(vec![Transition::new(
                    Duration::from_millis(200),
                    target,
                    Some(now),
                )]);
            } else {
                // Render the rest of the items as hidden.
                let max_no = stack_max_count + 1.;

                item.set_transitions(vec![
                    Transition::new(
                        Duration::from_millis(200),
                        PartialStyle {
                            x: Some(self.config.margin.x + max_no * self.config.stack.inset),
                            y: Some(top_y - self.config.stack.peek),

                            w: Some(self.config.width - 2. * max_no * self.config.stack.inset),
                            h: Some(2. * self.config.stack.peek),

                            outer_opacity: Some(0.),
                            inner_opacity: None,
                        },
                        Some(now),
                    ),
                    Transition::new(
                        Duration::from_millis(25),
                        PartialStyle {
                            inner_opacity: Some(0.),
                            ..Default::default()
                        },
                        Some(now),
                    ),
                ]);
            }
        }
    }

    /// The handler for when a pointer left click happens within the bounds of
    /// this stack.
    pub fn on_left_click(&mut self, _event: &PointerEvent) -> Vec<AppCommand> {
        vec![]
    }

    /// The handler for when a pointer right click happens within the bounds of
    /// this stack.
    pub fn on_right_click(&mut self, event: &PointerEvent) -> Vec<AppCommand> {
        if let Some(item) = self.find_at(event.position) {
            self.dismiss(item.id())
        } else {
            vec![]
        }
    }

    /// The handler for when a pointer is hovering within the bounds of this
    /// stack.
    pub fn on_hover(&mut self, event: &PointerEvent) -> Vec<AppCommand> {
        let item = self.find_at(event.position);

        if let Some(_) = item {
            let mut commands = vec![AppCommand::SetCursor(CursorIcon::Pointer)];

            if self.set_presentation(Presentation::List) {
                commands.push(AppCommand::Redraw);
            }

            commands
        } else {
            vec![]
        }
    }

    /// The handler for when the pointer leaves the bounds of this stack.
    /// This method is only expected to be called once after a leave happens.
    pub fn on_leave(&mut self) -> Vec<AppCommand> {
        self.set_presentation(Presentation::Stack);

        vec![
            AppCommand::Redraw,
            AppCommand::SetCursor(CursorIcon::Default),
        ]
    }

    // Renders the stack to the cairo canvas and returns a bool indicating if all the item
    // transitions have settled and the visual bounds of the stack.
    pub fn render(&mut self, cx: &cairo::Context) -> Result<(Option<Rect>, bool)> {
        let now = Instant::now();

        let mut settled = true;
        let mut full_bounds: Option<Rect> = None;
        let mut settled_dismissals = HashSet::<u32>::new();

        for (id, item) in self.items.iter_mut() {
            let item_settled = item.tick(&now);

            // Render and update the scene bounds.
            let bounds = item.render(cx).context("Could not render item: {id}")?;

            let fb = full_bounds.get_or_insert(bounds);
            fb.x1 = fb.x1.min(bounds.x1);
            fb.y1 = fb.y1.min(bounds.y1);
            fb.x2 = fb.x2.max(bounds.x2);
            fb.y2 = fb.y2.max(bounds.y2);

            // If the item was marked as dismissed, and the transition
            // around it has settled, then, remove.
            if item_settled && item.is_dimissed() {
                settled_dismissals.insert(*id);
            }

            settled &= item_settled;
        }

        if !settled_dismissals.is_empty() {
            self.items.retain(|id, _| !settled_dismissals.contains(id));
        }

        self.bounds = full_bounds;
        Ok((full_bounds, settled))
    }

    pub fn bounds(&self) -> Option<&Rect> {
        self.bounds.as_ref()
    }
}
