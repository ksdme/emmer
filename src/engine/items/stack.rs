use std::time::Duration;

use crate::{
    config,
    engine::items::{
        item::Item,
        style::{Style, Transition},
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

    pub fn set_mode(&mut self, config: &config::Config, mode: LayoutMode) {
        self.layout_mode = mode;
        self.layout(config);
    }

    // TODO: Lock the items.
    pub fn push(&mut self, config: &config::Config) {
        // Push a new item on the stack.
        let item = Item::new(Style {
            x: config.margin.x,
            y: -config.margin.y,
            w: config.width,
            h: 64.,
            opacity: 1.,
        });
        self.items.insert(0, item);

        // Re-layout the items.
        self.layout(config);
    }

    pub fn draw(&mut self, canvas: &skia_safe::Canvas) -> bool {
        let mut settled = false;
        for item in self.items.iter_mut().rev() {
            settled |= !item.draw(canvas);
        }

        settled
    }

    pub fn layout(&mut self, config: &config::Config) {
        match self.layout_mode {
            LayoutMode::Spread => self.layout_spread(config),
            LayoutMode::Stacked => self.layout_stacked(config),
        }
    }

    fn layout_spread(&mut self, config: &config::Config) {
        let mut top_y = config.margin.y;

        for (no, item) in self.items.iter_mut().enumerate() {
            // Show the first config.spread.max_count items.
            if no <= config.spread.max_count {
                let target = Style {
                    x: config.margin.x,
                    y: top_y,
                    w: config.width,
                    h: item.h,
                    opacity: 1.,
                };
                top_y = target.y + target.h + config.spread.gap;

                item.set_style(
                    None,
                    Transition::new(Duration::from_millis(250), target, None),
                );
            } else {
                // The rest of the items should naturally just go sit at the bottom.
                // It doesn't matter if all the other items sit are on top of each other
                // because they won't be visible.
                let target = Style {
                    x: config.margin.x,
                    y: top_y + config.spread.gap,
                    w: config.width,
                    h: item.h,
                    opacity: 0.,
                };

                item.set_style(
                    None,
                    // We are using a transition here instead of setting the value
                    // immediately so a new item will also act as expected.
                    Transition::new(Duration::from_millis(250), target, None),
                );
            }
        }
    }

    pub fn layout_stacked(&mut self, config: &config::Config) {
        let mut top_y = config.margin.y;

        for (no, item) in self.items.iter_mut().enumerate() {
            // Renders the first item as a regular block.
            if no == 0 {
                let target = Style {
                    x: config.margin.x,
                    y: top_y,
                    w: config.width,
                    h: item.h,
                    opacity: 1.,
                };
                top_y = target.y + target.h;

                item.set_style(
                    None,
                    Transition::new(Duration::from_millis(250), target, None),
                );
            } else if no <= config.stack.max_count {
                // Render the stack entries.
                let no = no as f32;

                let target = Style {
                    x: config.margin.x + no * config.stack.inset,
                    y: top_y - config.stack.peek,
                    w: config.width - 2. * no * config.stack.inset,
                    h: 2. * config.stack.peek,
                    opacity: 1.,
                };
                top_y = target.y + target.h;

                item.set_style(
                    None,
                    Transition::new(Duration::from_millis(250), target, None),
                );
            } else {
                // Render the rest of the items as hidden.
                let max_no = (config.stack.max_count + 1) as f32;

                let target = Style {
                    x: config.margin.x + max_no * config.stack.inset,
                    y: top_y - config.stack.peek,
                    w: config.width - 2. * max_no * config.stack.inset,
                    h: 2. * config.stack.peek,
                    opacity: 0.,
                };

                item.set_style(
                    None,
                    Transition::new(Duration::from_millis(250), target, None),
                );
            }
        }
    }
}
