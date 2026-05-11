use crate::{
    config::Theme,
    engine::items::style::{Style, Transition},
    renderer::draw::{
        self,
        block::{Block, Shadow},
    },
};

#[derive(Debug, PartialEq)]
pub enum State {
    Alive,
    Dismissed,
}

#[derive(Debug)]
pub struct Item {
    pub id: usize,
    pub state: State,

    // The height of each item is based on its contents. So, we need to
    // calculate and cache it. This might be >= style.h when the card is
    // transitioning.
    pub h: f32,

    style: Style,
    transition: Option<Transition>,
}

impl Item {
    pub fn new(id: usize, style: Style) -> Self {
        Self {
            id,
            state: State::Alive,

            h: style.h,

            style,
            transition: None,
        }
    }

    pub fn set_style(&mut self, current: Option<Style>, transition: Transition) {
        if let Some(current) = current {
            self.style = current;
        }

        self.transition = Some(transition);
    }

    /// Progresses the transition attached to the item if any and returns the updated
    /// visual state of the item and a boolean indicating if the transition has settled.
    pub fn draw(&mut self, theme: &Theme, canvas: &skia_safe::Canvas) -> bool {
        let settled = if let Some(transition) = &mut self.transition {
            let (style, settled) = transition.interpolate(&self.style, None);

            self.style = style;
            if settled {
                self.transition = None;
            }

            settled
        } else {
            // If there is no transition in progress and we are in a settled invisible
            // state, then, there is no point in trying to draw anything.
            if !self.style.visible() {
                return true;
            }

            true
        };

        draw::block(
            theme,
            &Block {
                shadow: Some(Shadow::SM),
                ..Default::default()
            },
            canvas,
            self.style.x,
            self.style.y,
            self.style.w,
            self.style.h,
            self.style.opacity,
        );

        settled
    }

    pub fn hitbox(&self) -> Option<(f32, f32, f32, f32)> {
        if self.style.visible() {
            Some((
                self.style.x,
                self.style.y,
                self.style.x + self.style.w,
                self.style.y + self.style.h,
            ))
        } else {
            None
        }
    }
}
