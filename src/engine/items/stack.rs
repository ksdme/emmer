use std::time::Duration;

use log::{debug, info};

use crate::{
    config::ComputedConfig,
    engine::items::{
        item::{Item, State},
        style::{PartialStyle, Style, Transition},
    },
};

#[derive(Clone, Debug)]
pub enum LayoutMode {
    Spread,
    Stacked,
}

/// The container for incoming items.
pub struct Stack {
    // TODO: Switch to something more efficient like a linked list.
    // We need pushing to the top and efficient removal from middle.
    items: Vec<Item>,

    // The layout mode the stack is currently rendering in.
    layout_mode: LayoutMode,
}

impl Stack {
    pub fn new() -> Self {
        Self {
            items: vec![],

            layout_mode: LayoutMode::Stacked,
        }
    }

    pub fn set_mode(&mut self, config: &ComputedConfig, mode: LayoutMode) {
        self.layout_mode = mode;
        info!(target: "stack", "set_mode: {:?}", &self.layout_mode);

        self.layout(config);
    }

    // TODO: Lock the items.
    /// Push a new item on the stack.
    pub fn push(&mut self, config: &ComputedConfig) {
        let item = Item::new(
            config,
            self.items.len(),
            Style {
                x: config.margin.x,
                y: -config.margin.y,
                w: config.width,
                h: rand::random_range(64.0..80.0),
                box_opacity: 1.,
                text_opacity: 1.,
            },
        );

        info!(target: "stack", "push: {:?}", &item.id);
        self.items.insert(0, item);

        self.layout(config);
    }

    // TODO: Lock the items.
    /// Remove an item from the stack.
    pub fn dismiss(&mut self, config: &ComputedConfig, at: (f32, f32)) {
        let item = self.items.iter_mut().find(|item| match item.hitbox() {
            Some((x1, y1, x2, y2)) => {
                let (x, y) = at;
                x >= x1 && y >= y1 && x <= x2 && y <= y2
            }
            None => false,
        });

        if let Some(item) = item {
            info!(target: "stack", "dismissing: {:?}", &item.id);
            item.state = State::Dismissed;
        } else {
            info!(target: "stack", "dismiss item not resolved");
        }

        self.layout(config);
    }

    pub fn draw(&mut self, config: &ComputedConfig, canvas: &skia_safe::Canvas) -> bool {
        let mut pending = false;

        for no in (0..self.items.len()).rev() {
            if let Some(item) = self.items.get_mut(no) {
                let settled = item.draw(config, canvas);

                // If the item was marked as dismissed, and the transition
                // around it has settled, then, remove it from memory.
                if settled && item.state == State::Dismissed {
                    self.items.remove(no);
                }

                pending |= !settled;
            }
        }

        pending
    }

    pub fn layout(&mut self, config: &ComputedConfig) {
        match self.layout_mode {
            LayoutMode::Spread => self.layout_spread(config),
            LayoutMode::Stacked => self.layout_stack(config),
        }
    }

    fn layout_spread(&mut self, config: &ComputedConfig) {
        debug!(target: "stack", "re-layout in spread mode");

        let mut no = 0;
        let mut top_y = config.margin.y;
        for item in self.items.iter_mut() {
            let (item_w, item_h) = item.size(config);

            // Show the first config.spread.max_count items.
            if no <= config.spread.max_count {
                let target = Style {
                    x: config.margin.x,
                    y: top_y,
                    w: item_w,
                    h: item_h,
                    box_opacity: match item.state {
                        State::Alive => 1.,
                        State::Dismissed => 0.,
                    },
                    text_opacity: match item.state {
                        State::Alive => 1.,
                        State::Dismissed => 0.,
                    },
                };

                if item.state == State::Alive {
                    no += 1;
                    top_y = target.y + target.h + config.spread.gap;
                }

                item.set_style(
                    None,
                    vec![Transition::new(
                        Duration::from_millis(500),
                        target.into(),
                        None,
                    )],
                );
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

                item.set_style(
                    None,
                    // We are using a transition here instead of setting the value
                    // immediately so a new item will also act as expected.
                    vec![Transition::new(
                        Duration::from_millis(500),
                        target.into(),
                        None,
                    )],
                );
            }
        }
    }

    pub fn layout_stack(&mut self, config: &ComputedConfig) {
        debug!(target: "stack", "re-layout in stack mode");

        let mut no = 0;
        let mut top_y = config.margin.y;
        for item in self.items.iter_mut() {
            let (item_w, item_h) = item.size(config);

            // Renders the first item as a regular block.
            if no == 0 {
                let target = Style {
                    x: config.margin.x,
                    y: top_y,
                    w: item_w,
                    h: item_h,
                    box_opacity: match item.state {
                        State::Alive => 1.,
                        State::Dismissed => 0.,
                    },
                    text_opacity: match item.state {
                        State::Alive => 1.,
                        State::Dismissed => 0.,
                    },
                };

                if item.state == State::Alive {
                    no += 1;
                    top_y = target.y + target.h;
                }

                item.set_style(
                    None,
                    vec![Transition::new(
                        Duration::from_millis(500),
                        target.into(),
                        None,
                    )],
                );
            } else if no < config.stack.max_count {
                // Render the stack entries.
                let target = PartialStyle {
                    x: Some(config.margin.x + (no as f32) * config.stack.inset),
                    y: Some(top_y - config.stack.peek),
                    w: Some(config.width - 2. * (no as f32) * config.stack.inset),
                    h: Some(2. * config.stack.peek),
                    box_opacity: Some(match item.state {
                        State::Alive => 1.,
                        State::Dismissed => 0.,
                    }),
                    text_opacity: None,
                };
                let target_text = PartialStyle {
                    text_opacity: Some(0.),
                    ..Default::default()
                };

                if item.state == State::Alive {
                    no += 1;
                    top_y = target.y.unwrap_or_default() + target.h.unwrap_or_default();
                }

                item.set_style(
                    None,
                    vec![
                        Transition::new(Duration::from_millis(500), target, None),
                        Transition::new(Duration::from_millis(25), target_text, None),
                    ],
                );
            } else {
                // Render the rest of the items as hidden.
                let max_no = (config.stack.max_count + 1) as f32;

                let target = Style {
                    x: config.margin.x + max_no * config.stack.inset,
                    y: top_y - config.stack.peek,
                    w: config.width - 2. * max_no * config.stack.inset,
                    h: 2. * config.stack.peek,
                    box_opacity: 0.,
                    text_opacity: 0.,
                };

                item.set_style(
                    None,
                    vec![Transition::new(
                        Duration::from_millis(500),
                        target.into(),
                        None,
                    )],
                );
            }
        }
    }
}
