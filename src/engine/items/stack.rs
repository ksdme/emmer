use crate::{
    config::Config,
    engine::items::{
        item::{Item, State},
        style::Style,
    },
};

/// The container for incoming items.
pub struct Stack {
    // TODO: Switch to something more efficient like a linked list.
    // We need pushing to the top and efficient removal from middle.
    items: Vec<Item>,

    //
    spread: bool,
}

impl Stack {
    pub fn new() -> Self {
        Self {
            items: vec![],
            spread: false,
        }
    }

    pub fn push(&mut self, config: &Config) {
        // Transition all the items downwards.
        let item = Item::new(Style::default());
        self.items.insert(0, item);

        let mut no: u8 = 0;
        let mut target_y = config.margin.y;

        for item in self.items.iter_mut() {
            if no < config.stack.max_count {
                if item.state != State::Dismissed {
                    target_y = item.transition_to(
                        State::Visible(no, self.spread),
                        &config,
                        target_y,
                        target_y,
                    );
                }

                no += 1;
            } else {
                if item.state != State::Dismissed {
                    target_y = item.transition_to(State::Overflown, &config, target_y, target_y);
                }
            }
        }
    }

    pub fn dismiss(&mut self, config: &Config, at: (f32, f32)) {
        let mut no: u8 = 0;

        let mut target_y = config.margin.y;
        let mut current_y = config.margin.y;

        for item in self.items.iter_mut() {
            let (xhit, yhit) = at;
            let (x1, y1, x2, y2) = item.hitbox();

            let hit = xhit >= x1 && xhit <= x2 && yhit >= y1 && yhit <= y2;
            if hit && matches!(item.state, State::Visible(_, _)) {
                current_y = item.transition_to(State::Dismissed, &config, current_y, target_y);
            }
            if matches!(item.state, State::Dismissed) {
                continue;
            }

            if no < config.stack.max_count {
                target_y = item.transition_to(
                    State::Visible(no, self.spread),
                    &config,
                    current_y,
                    target_y,
                );
            } else {
                target_y = item.transition_to(State::Dismissed, &config, current_y, target_y);
            }

            no += 1;
        }
    }

    pub fn set_spread(&mut self, config: &Config, spread: bool) {
        self.spread = spread;

        let mut target_y = config.margin.y;
        for item in self.items.iter_mut() {
            target_y = match item.state.clone() {
                State::Visible(no, _) => {
                    item.transition_to(State::Visible(no, spread), &config, target_y, target_y)
                }
                state => item.transition_to(state, &config, target_y, target_y),
            };
        }
    }

    pub fn draw(&mut self, canvas: &skia_safe::Canvas) -> bool {
        let mut settled = false;
        for item in self.items.iter_mut().rev() {
            settled |= !item.draw(canvas);
        }
        settled
    }
}
