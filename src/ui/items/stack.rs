use anyhow::{Context, Result};
use smithay_client_toolkit::seat::pointer::{CursorIcon, PointerEvent};

use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
    time::Instant,
};

use crate::{
    config::ComputedConfig,
    dbus::CloseReason,
    notification::Notification,
    ui::{
        items::{Item, item::VisualState},
        renderables::Rect,
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
    Spread,
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

    // TODO: This needs to be as efficient as possible.
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
        let item = Item::spawn(notification, self.config.clone(), &self.presentation);

        self.items.insert(item.id(), item);
        self.update_visual_states();

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
            self.update_visual_states();

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
            self.update_visual_states();

            true
        } else {
            false
        }
    }

    fn update_visual_states(&mut self) {
        let now = Instant::now();

        match self.presentation {
            Presentation::Stack => {
                let mut no = 0;
                let mut top_y = self.config.margin.y;

                for (_, item) in self.items.iter_mut().rev() {
                    if no < self.config.stack.max_count {
                        let y =
                            item.set_visual_state(VisualState::Stacked { pos: no, y: top_y }, now);

                        // If an item is dismissed, then we expect that the next item replaces
                        // its visual position.
                        if !item.is_dimissed() {
                            no += 1;
                            top_y = y;
                        }
                    } else {
                        let _ = item.set_visual_state(VisualState::Hidden { y: top_y }, now);
                    }
                }
            }

            Presentation::Spread => {
                let mut no = 0;
                let mut top_y = self.config.margin.y;

                for (_, item) in self.items.iter_mut().rev() {
                    if no < self.config.spread.max_count {
                        let y = item.set_visual_state(VisualState::Spread { y: top_y }, now);

                        // If an item is dismissed, then we expect that the next item replaces
                        // its visual position.
                        if !item.is_dimissed() {
                            no += 1;
                            top_y = y;
                        }
                    } else {
                        let _ = item.set_visual_state(VisualState::Hidden { y: top_y }, now);
                    }
                }
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
        if let Some(id) = self.find_at(event.position).map(|el| el.id()) {
            self.dismiss(id)
        } else {
            vec![]
        }
    }

    /// The handler for when a pointer is hovering within the bounds of this
    /// stack.
    pub fn on_hover(&mut self, event: &PointerEvent) -> Vec<AppCommand> {
        if let Some(_) = self.find_at(event.position) {
            let mut commands = vec![AppCommand::SetCursor(CursorIcon::Pointer)];

            if self.set_presentation(Presentation::Spread) {
                commands.push(AppCommand::Redraw);
            }

            commands
        } else {
            vec![AppCommand::SetCursor(CursorIcon::Default)]
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
            if let Some(bounds) = item.render(cx).context("Could not render item: {id}")? {
                let fb = full_bounds.get_or_insert(bounds);
                fb.x1 = fb.x1.min(bounds.x1);
                fb.y1 = fb.y1.min(bounds.y1);
                fb.x2 = fb.x2.max(bounds.x2);
                fb.y2 = fb.y2.max(bounds.y2);
            }

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
