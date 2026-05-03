use std::time::{Duration, Instant};

use crate::{
    config::Config,
    engine::items::style::{Style, Transition},
    renderer::draw::{
        self,
        block::{Block, Shadow},
    },
};

#[derive(Clone, PartialEq, Debug)]
pub enum State {
    Visible(u8, bool),
    Overflown,
    Dismissed,
}

pub struct Item {
    pub state: State,

    // The height of each item is based on its contents. So, we need to
    // calculate and cache it. This might be >= style.h when the card is
    // transitioning.
    pub h: f32,

    style: Style,
    transition: Option<Transition>,
}

impl Item {
    pub fn new(style: Style) -> Self {
        Self {
            state: State::Overflown,

            h: 60.,

            style,
            transition: None,
        }
    }

    /// Progresses the transition attached to the item if any and returns the updated
    /// visual state of the item and a boolean indicating if the transition has settled.
    pub fn tick(&mut self, now: Option<Instant>) -> (&Style, bool) {
        if let Some(transition) = &self.transition {
            let (state, done) = transition.interpolate(&self.style, now);

            self.style = state;
            if done {
                self.transition = None;
            }

            (&self.style, done)
        } else {
            (&self.style, true)
        }
    }

    pub fn transition_to(
        &mut self,
        to: State,
        config: &Config,
        current_y: f32,
        target_y: f32,
    ) -> f32 {
        if self.state == to {
            return target_y;
        }

        let (initial_style, target_style, target_y) = match (&self.state, to.clone()) {
            (State::Overflown, State::Visible(no, true)) => (
                Some(Style {
                    x: config.margin.x,
                    y: current_y + config.spread.gap - if no == 0 { config.stack.peek } else { 0. },
                    w: config.width,
                    h: self.h,
                    opacity: 0.,
                }),
                Style {
                    x: config.margin.x,
                    y: target_y + config.spread.gap,
                    w: config.width,
                    h: self.h,
                    opacity: 1.,
                },
                target_y + config.spread.gap + self.h,
            ),

            (State::Visible(_, _), State::Visible(pos, spread)) => {
                let pos = pos as f32;

                if spread {
                    (
                        None,
                        Style {
                            x: config.margin.x,
                            y: target_y + config.spread.gap,
                            w: config.width,
                            h: self.h,
                            opacity: 1.,
                        },
                        target_y + config.spread.gap + self.h,
                    )
                } else {
                    (
                        None,
                        Style {
                            x: config.margin.x + pos * config.stack.inset,
                            y: target_y + config.stack.peek,
                            h: self.h,
                            w: config.width - 2. * pos * config.stack.inset,
                            opacity: 1.,
                        },
                        target_y + config.stack.peek,
                    )
                }
            }

            (State::Visible(_, _), State::Overflown) => {
                let mut style = self.style.clone();
                style.y += 2. * config.stack.peek;
                style.opacity = 0.;
                (None, style, target_y)
            }

            (State::Visible(_, _), State::Dismissed) => {
                let mut style = self.style.clone();
                style.opacity = 0.;
                (None, style, target_y + self.h)
            }

            _ => {
                unreachable!();
            }
        };

        self.state = to;
        if let Some(initial_style) = initial_style {
            self.style = initial_style;
        }
        // TODO: Is there a scenario where we should carry over the duration?
        self.transition = Some(Transition::new(
            Duration::from_millis(1000),
            target_style.clone(),
        ));

        target_y
    }

    pub fn draw(&mut self, canvas: &skia_safe::Canvas) -> bool {
        let settled = if let Some(transition) = &mut self.transition {
            let (style, settled) = transition.interpolate(&self.style, None);

            self.style = style;
            if settled {
                self.transition = None;
            }

            settled
        } else {
            true
        };

        draw::block(
            canvas,
            &Block {
                shadow: Some(Shadow::SM),
                ..Default::default()
            },
            self.style.x,
            self.style.y,
            self.style.w,
            self.style.h,
            self.style.opacity,
        );

        settled
    }

    pub fn hitbox(&self) -> (f32, f32, f32, f32) {
        (
            self.style.x,
            self.style.y,
            self.style.x + self.style.w,
            self.style.y + self.style.h,
        )
    }
}
