use std::{
    collections::{BTreeMap, HashSet},
    time::{Duration, Instant},
};

use skia_safe::{Contains, Point, Rect};

use crate::{
    config::ComputedConfig,
    notification::Notification,
    ui::items::{
        item::{Item, State},
        style::{PartialStyle, Style, Transition},
    },
};

#[derive(Debug)]
pub enum DismissReason {
    Manual,
    Expired,
}

#[derive(Debug)]
pub enum StackCommand {
    Redraw,
    NotifyDismissed(u32, DismissReason),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LayoutMode {
    Spread,
    Stacked,
}

/// The container for incoming items.
pub struct Stack {
    // TODO: Switch to something more efficient like a linked list.
    // We need pushing to the top and efficient removal from middle.
    items: BTreeMap<u32, Item>,

    // The layout mode the stack is currently rendering in.
    layout_mode: LayoutMode,
}

impl Stack {
    pub fn new() -> Self {
        Self {
            items: BTreeMap::new(),

            layout_mode: LayoutMode::Stacked,
        }
    }

    // Finds an item that is at (x, y) position.
    pub fn find_at(&self, at: (f32, f32)) -> Option<u32> {
        self.items
            .values()
            .into_iter()
            .find(|el| {
                if let Some(hitbox) = el.hitbox() {
                    hitbox.contains(Point::from(at))
                } else {
                    false
                }
            })
            .map(|el| el.id())
    }

    pub fn set_layout_mode(
        &mut self,
        config: &ComputedConfig,
        mode: LayoutMode,
    ) -> Vec<StackCommand> {
        if self.layout_mode != mode {
            log::info!("stack.set_layout_mode: {:?}", mode);

            self.layout_mode = mode;
            self.layout(config);

            vec![StackCommand::Redraw]
        } else {
            vec![]
        }
    }

    // TODO: Lock the items.
    /// Push a new item on the stack.
    pub fn push(
        &mut self,
        config: &ComputedConfig,
        notification: Notification,
    ) -> Vec<StackCommand> {
        log::info!("stack.push: {:?}", notification.id());
        let mut item = Item::new(config, notification);

        let (_, h) = item.content_size(config);
        item.set_style(Style {
            x: config.margin.x,
            y: match self.layout_mode {
                LayoutMode::Spread => config.margin.y - config.spread.gap - h,
                LayoutMode::Stacked => -config.margin.y,
            },

            w: config.width,
            h,

            box_opacity: 1.,
            text_opacity: 1.,
        });

        self.items.insert(item.id(), item);
        self.layout(config);

        vec![StackCommand::Redraw]
    }

    // TODO: Lock the items.
    /// Remove an item from the stack.
    pub fn dismiss(
        &mut self,
        config: &ComputedConfig,
        id: u32,
        reason: DismissReason,
    ) -> Vec<StackCommand> {
        match self.items.get_mut(&id) {
            Some(item) if item.state() != State::Dismissed => {
                item.set_state(State::Dismissed);
                self.layout(config);

                vec![
                    StackCommand::NotifyDismissed(id, reason),
                    StackCommand::Redraw,
                ]
            }
            _ => vec![],
        }
    }

    pub fn dismiss_expired(&mut self, config: &ComputedConfig) -> Vec<StackCommand> {
        let mut commands = vec![];

        for item in self.items.values_mut() {
            if item.notification().is_expired() && item.state() != State::Dismissed {
                item.set_state(State::Dismissed);

                commands.push(StackCommand::NotifyDismissed(
                    item.id(),
                    DismissReason::Expired,
                ));
            }
        }

        if !commands.is_empty() {
            self.layout(config);
            commands.push(StackCommand::Redraw);
        }

        commands
    }

    pub fn layout(&mut self, config: &ComputedConfig) {
        let now = Instant::now();
        match self.layout_mode {
            LayoutMode::Spread => self.layout_spread(config, now),
            LayoutMode::Stacked => self.layout_stack(config, now),
        }
    }

    fn layout_spread(&mut self, config: &ComputedConfig, now: Instant) {
        let mut no = 0;
        let mut top_y = config.margin.y;

        for (_, item) in self.items.iter_mut().rev() {
            let item_state = item.state();
            let (item_w, item_h) = item.content_size(config);

            // Show the first config.spread.max_count items.
            if no <= config.spread.max_count {
                let target = Style {
                    x: config.margin.x,
                    y: match item_state {
                        State::Alive => top_y,
                        State::Dismissed => top_y - item_h,
                    },

                    w: item_w,
                    h: item_h,

                    box_opacity: match item_state {
                        State::Alive => 1.,
                        State::Dismissed => 0.,
                    },
                    text_opacity: match item_state {
                        State::Alive => 1.,
                        State::Dismissed => 0.,
                    },
                };

                if item_state == State::Alive {
                    no += 1;
                    top_y = top_y + target.h + config.spread.gap;
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
                    x: config.margin.x,
                    y: top_y + config.spread.gap,

                    w: item_w,
                    h: item_h,

                    box_opacity: 0.,
                    text_opacity: 0.,
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

    pub fn layout_stack(&mut self, config: &ComputedConfig, now: Instant) {
        let mut no = 0;
        let mut top_y = config.margin.y;

        for (_, item) in self.items.iter_mut().rev() {
            let item_state = item.state();
            let (item_w, item_h) = item.content_size(config);

            // Renders the first item as a regular block.
            if no == 0 {
                let target = Style {
                    x: config.margin.x,
                    y: top_y,

                    w: item_w,
                    h: item_h,

                    box_opacity: match item_state {
                        State::Alive => 1.,
                        State::Dismissed => 0.,
                    },
                    text_opacity: match item_state {
                        State::Alive => 1.,
                        State::Dismissed => 0.,
                    },
                };

                if item_state == State::Alive {
                    no += 1;
                    top_y = target.y + target.h;
                }

                item.set_transitions(vec![Transition::new(
                    Duration::from_millis(200),
                    target.into(),
                    Some(now),
                )]);
            } else if no < config.stack.max_count {
                // Render the stack entries.

                // The height of the card should be smaller than the top-most card.
                let h = item_h.min(top_y - config.margin.y);
                let target = PartialStyle {
                    x: Some(config.margin.x + (no as f32) * config.stack.inset),
                    y: Some(top_y + config.stack.peek - h),

                    w: Some(config.width - 2. * (no as f32) * config.stack.inset),
                    h: Some(h),

                    box_opacity: Some(match item_state {
                        State::Alive => 1.,
                        State::Dismissed => 0.,
                    }),
                    text_opacity: Some(0.),
                };

                if item_state == State::Alive {
                    no += 1;
                    top_y = target.y.unwrap_or_default() + target.h.unwrap_or_default();
                }

                item.set_transitions(vec![Transition::new(
                    Duration::from_millis(200),
                    target,
                    Some(now),
                )]);
            } else {
                // Render the rest of the items as hidden.
                let max_no = (config.stack.max_count + 1) as f32;

                item.set_transitions(vec![
                    Transition::new(
                        Duration::from_millis(200),
                        PartialStyle {
                            x: Some(config.margin.x + max_no * config.stack.inset),
                            y: Some(top_y - config.stack.peek),

                            w: Some(config.width - 2. * max_no * config.stack.inset),
                            h: Some(2. * config.stack.peek),

                            box_opacity: Some(0.),
                            text_opacity: None,
                        },
                        Some(now),
                    ),
                    Transition::new(
                        Duration::from_millis(25),
                        PartialStyle {
                            text_opacity: Some(0.),
                            ..Default::default()
                        },
                        Some(now),
                    ),
                ]);
            }
        }
    }

    // Renders the stack to the skia canvas and returns a bool indicating if all the item transitions
    // have settled and the visual bounds of the stack.
    pub fn render(&mut self, config: &ComputedConfig, canvas: &skia_safe::Canvas) -> (bool, Rect) {
        let now = Instant::now();

        let mut settled = true;
        let mut settled_dismissals = HashSet::<u32>::new();
        let (mut x1, mut y1, mut x2, mut y2) = (0f32, 0f32, 0f32, 0f32);

        for (id, item) in self.items.iter_mut() {
            let item_settled = item.tick(&now);

            // Render and update the scene bounds.
            let rect = item.render(config, canvas);
            x1 = x1.min(rect.left());
            y1 = y1.min(rect.top());
            x2 = x2.max(rect.right());
            y2 = y2.max(rect.bottom());

            // If the item was marked as dismissed, and the transition
            // around it has settled, then, remove.
            if item_settled && item.state() == State::Dismissed {
                settled_dismissals.insert(id.clone());
            }

            settled &= item_settled;
        }

        if !settled_dismissals.is_empty() {
            self.items.retain(|id, _| !settled_dismissals.contains(id));
        }

        (settled, Rect::from_ltrb(x1, y1, x2, y2))
    }
}
