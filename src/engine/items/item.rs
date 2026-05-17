use skia_safe::textlayout::{Paragraph, ParagraphBuilder, ParagraphStyle};

use crate::{
    config::ComputedConfig,
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

    style: Style,
    transitions: Vec<Transition>,

    render_cache: ItemRenderCache,
}

impl Item {
    pub fn new(config: &ComputedConfig, id: usize, style: Style) -> Self {
        Self {
            id,
            state: State::Alive,

            style,
            transitions: vec![],

            render_cache: ItemRenderCache::new(config),
        }
    }

    /// Progresses the transition attached to the item if any and returns the updated
    /// visual state of the item and a boolean indicating if the transition has settled.
    pub fn draw(&mut self, config: &ComputedConfig, canvas: &skia_safe::Canvas) -> bool {
        // Progress all transitions on the item.
        self.transitions.retain(|transition| {
            let (style, settled) = transition.interpolate(&self.style, None);
            self.style = style;
            !settled
        });

        let settled = self.transitions.is_empty();
        if !self.style.visible() {
            return settled;
        }

        draw::block(
            &Block {
                shadow: Some(Shadow::SM),
                ..Default::default()
            },
            canvas,
            self.style.x,
            self.style.y,
            self.style.w,
            self.style.h,
            self.style.box_opacity,
        );

        if self.style.text_opacity > 0. {
            canvas.save_layer_alpha_f(None, self.style.text_opacity);

            // Draw the title.
            self.render_cache.title_p.paint(
                canvas,
                (
                    self.style.x + config.padding.x,
                    self.style.y + config.padding.y,
                ),
            );

            // Draw the body.
            self.render_cache.body_p.paint(
                canvas,
                (
                    self.style.x + config.padding.x,
                    self.style.y + config.padding.y + self.render_cache.title_p.height() + 8.,
                ),
            );

            canvas.restore();
        }

        settled
    }

    pub fn set_style(&mut self, current: Option<Style>, transitions: Vec<Transition>) {
        if let Some(current) = current {
            self.style = current;
        }

        self.transitions = transitions;
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

    pub fn size(&self, config: &ComputedConfig) -> (f32, f32) {
        (
            config.width,
            (2. * config.padding.y) + self.render_cache.content_height(),
        )
    }
}

#[derive(Debug)]
struct ItemRenderCache {
    title_p: Paragraph,
    body_p: Paragraph,
}

impl ItemRenderCache {
    pub fn new(config: &ComputedConfig) -> Self {
        let w = config.width - 2. * config.padding.x;

        let mut p_style = ParagraphStyle::default();
        p_style.set_max_lines(4);

        let mut title_p = ParagraphBuilder::new(&p_style, config.theme.font_collection.clone())
            .push_style(&config.theme.title_style)
            .add_text("Hello")
            .build();
        title_p.layout(w);

        let mut body_p = ParagraphBuilder::new(&p_style, config.theme.font_collection.clone())
            .push_style(&config.theme.body_style)
            .add_text("Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.")
            .build();
        body_p.layout(w);

        Self { title_p, body_p }
    }

    pub fn content_height(&self) -> f32 {
        self.title_p.height() + self.body_p.height() + 8.
    }
}
