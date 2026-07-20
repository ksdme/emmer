use anyhow::{Context, Result};
use smithay_client_toolkit::seat::pointer::{CursorIcon, PointerEvent};

use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
    time::Instant,
};

use crate::{
    config::ComputedConfig,
    dbus::{notification::Notification, service::CloseReason},
    ui::{
        anchors::{Anchor, VerticalAnchor},
        items::Item,
        renderables::Rect,
    },
};

/// The presentation of the current stack.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Presentation {
    Stacked,
    Spread,
}

/// The order of the item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Order {
    Visible(usize, f64),
    Overflown(usize, f64),
}

#[derive(Debug)]
pub enum AppCommand {
    Redraw,
    SetCursor(CursorIcon),
    NotifyClosed(u32, CloseReason),
    NotifyAction(u32, String),
}

/// The container for the items.
pub struct Stack {
    config: Arc<ComputedConfig>,
    anchor: Anchor,
    presentation: Presentation,

    items: BTreeMap<u32, Item>,

    bounds: Option<Rect>,
    hovering: Option<u32>,
}

impl Stack {
    pub fn new(config: Arc<ComputedConfig>, anchor: Anchor) -> Self {
        Self {
            config,
            anchor,
            presentation: Presentation::Stacked,

            items: BTreeMap::new(),

            bounds: None,
            hovering: None,
        }
    }

    /// Update the anchor of the stack.
    pub fn set_anchor(&mut self, anchor: Anchor) {
        self.anchor = anchor;
        self.update_visual_states();
    }

    // TODO: This needs to be as efficient as possible.
    /// Finds an item that is at (x, y) visual position on this stack.
    pub fn find_at_mut(&mut self, at: (f64, f64)) -> Option<&mut Item> {
        for el in self.items.values_mut().rev() {
            if let Some(hitbox) = el.bounds() {
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

    // TODO: Lock the items.
    /// Pushes an item to the stack and returns a list of resulting side effects.
    pub fn push(&mut self, notification: Notification) -> Result<Vec<AppCommand>> {
        log::info!("stack.push: {:?}", notification.id);
        let item = Item::spawn(
            notification,
            self.config.clone(),
            &self.anchor,
            self.presentation,
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

        for item in self.items.values_mut() {
            if item.is_expired() && item.dismiss() {
                commands.push(AppCommand::NotifyClosed(item.id(), CloseReason::Expired));
            }
        }

        if !commands.is_empty() {
            self.update_visual_states();
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
            // When we have only one item, trying to transition visual state here
            // will only make it look jarring to look at.
            if self.items.len() > 1 {
                self.update_visual_states();
            }

            true
        } else {
            false
        }
    }

    fn update_visual_states(&mut self) {
        let now = Some(Instant::now());

        let mut y = match self.anchor.vertical {
            VerticalAnchor::Top => self.config.margin.y,
            VerticalAnchor::Bottom => self.anchor.layer_height - self.config.margin.y,
        };

        match self.presentation {
            Presentation::Stacked => {
                let mut no = 0;

                for (_, item) in self.items.iter_mut().rev() {
                    if no < self.config.stack.max_count {
                        y = item.set_visual_state(
                            &self.anchor,
                            Presentation::Stacked,
                            Order::Visible(no, y),
                            now,
                        );

                        // If an item is dismissed, we expect that the next item replaces its
                        // visual position.
                        if !item.was_dismissed() {
                            no += 1;
                        }
                    } else {
                        y = item.set_visual_state(
                            &self.anchor,
                            Presentation::Stacked,
                            Order::Overflown(no, y),
                            now,
                        );
                    }
                }
            }

            Presentation::Spread => {
                let mut no = 0;

                for (_, item) in self.items.iter_mut().rev() {
                    if no < self.config.spread.max_count {
                        y = item.set_visual_state(
                            &self.anchor,
                            Presentation::Spread,
                            Order::Visible(no, y),
                            now,
                        );

                        // If an item is dismissed, we expect that the next item replaces its
                        // visual position.
                        if !item.was_dismissed() {
                            no += 1;
                        }
                    } else {
                        y = item.set_visual_state(
                            &self.anchor,
                            Presentation::Spread,
                            Order::Overflown(no, y),
                            now,
                        );
                    }
                }
            }
        }
    }

    /// The handler for when a pointer is hovering within the bounds of this
    /// stack.
    pub fn on_hover(&mut self, event: &PointerEvent) -> Vec<AppCommand> {
        // Trigger on_hover on the current item.
        let (hit, mut commands) = if let Some(item) = self.find_at_mut(event.position) {
            let id = item.id();
            let mut commands = item.on_hover(event);

            if self.set_presentation(Presentation::Spread) {
                // TODO: item.on_hover might have queued a redraw.
                // But, this is not a huge problem because the double draw happens only
                // when during the presentation transition.
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

        commands
    }

    /// The handler for when the pointer leaves the bounds of this stack.
    /// This method is only expected to be called once after a leave happens.
    pub fn on_leave(&mut self) -> Vec<AppCommand> {
        self.set_presentation(Presentation::Stacked);

        vec![
            AppCommand::Redraw,
            AppCommand::SetCursor(CursorIcon::Default),
        ]
    }

    /// The handler for when a pointer left click happens within the bounds of
    /// this stack.
    pub fn on_left_click(&mut self, event: &PointerEvent) -> Vec<AppCommand> {
        if let Some(item) = self.find_at_mut(event.position) {
            let (mut commands, update_visual_states) = item.on_left_click(event);

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
        if let Some(item) = self.find_at_mut(event.position) {
            let (mut commands, update_visual_states) = item.on_right_click(event);

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
    // transitions have settled and the visual bounds of the stack.
    pub fn render(&mut self, cx: &cairo::Context) -> Result<(Option<Rect>, bool)> {
        let now = Instant::now();

        let mut settled = true;
        let mut full_bounds: Option<Rect> = None;
        let mut settled_dismissals = HashSet::<u32>::new();

        for (id, item) in self.items.iter_mut() {
            let item_settled = item.tick(&now);

            // Render and update the scene bounds.
            if let Some(bounds) = item
                .render(
                    cx,
                    match self.anchor.vertical {
                        VerticalAnchor::Top => VerticalAnchor::Bottom,
                        VerticalAnchor::Bottom => VerticalAnchor::Top,
                    },
                )
                .context("Could not render item: {id}")?
            {
                let fb = full_bounds.get_or_insert(bounds);
                fb.x1 = fb.x1.min(bounds.x1);
                fb.y1 = fb.y1.min(bounds.y1);
                fb.x2 = fb.x2.max(bounds.x2);
                fb.y2 = fb.y2.max(bounds.y2);
            }

            // If the item was marked as dismissed, and the transition
            // around it has settled, then, remove.
            if item_settled && item.was_dismissed() {
                settled_dismissals.insert(*id);
            }

            settled &= item_settled;
        }

        if !settled_dismissals.is_empty() {
            self.items.retain(|id, _| !settled_dismissals.contains(id));
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

        self.bounds = full_bounds;
        Ok((full_bounds, settled))
    }

    pub fn bounds(&self) -> Option<&Rect> {
        self.bounds.as_ref()
    }
}
