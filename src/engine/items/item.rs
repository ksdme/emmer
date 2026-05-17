use std::time::Instant;

use skia_safe::{
    ClipOp, Rect,
    textlayout::{Paragraph, ParagraphBuilder, ParagraphStyle},
};

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
    pub fn new(config: &ComputedConfig, id: usize) -> Self {
        Self {
            id,
            state: State::Alive,

            style: Style::default(),
            transitions: vec![],

            render_cache: ItemRenderCache::new(config),
        }
    }

    /// Progresses the transition attached to the item if any and a boolean indicating
    /// if all the transitions have settled.
    pub fn tick(&mut self, now: &Instant) -> bool {
        self.transitions.retain_mut(|transition| {
            let (style, settled) = transition.interpolate(&self.style, now);
            self.style = style;
            !settled
        });

        self.transitions.is_empty()
    }

    /// Renders the current item to a skia canvas.
    pub fn render(&mut self, config: &ComputedConfig, canvas: &skia_safe::Canvas) {
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
            // We do this because the text style is baked into the render cache during
            // the object construction.
            canvas.save_layer_alpha_f(None, self.style.text_opacity);

            let avail_h = self.style.h - 2. * config.padding.y;
            let content_h = self.render_cache.content_height();
            let y = if content_h > avail_h {
                // Clip the overflowing region.
                canvas.clip_rect(
                    Rect::from_xywh(
                        self.style.x + config.padding.x,
                        self.style.y + config.padding.y,
                        self.style.w - 2. * config.padding.x,
                        self.style.h - 2. * config.padding.y,
                    ),
                    ClipOp::Intersect,
                    false,
                );

                // Anchor the content to the bottom of the box. Since the clip is a
                // fixed window, the easiest way to anchor is to do it during the draw.
                // TODO: Does bottom anchoring work in all cases?
                self.style.y + config.padding.y - (content_h - avail_h)
            } else {
                self.style.y + config.padding.y
            };

            // Draw the title.
            self.render_cache
                .title_p
                .paint(canvas, (self.style.x + config.padding.x, y));

            // Draw the body.
            self.render_cache.body_p.paint(
                canvas,
                (
                    self.style.x + config.padding.x,
                    y + self.render_cache.title_p.height() + 8.,
                ),
            );

            canvas.restore();
        }
    }

    pub fn set_style(&mut self, style: Style) {
        self.style = style;
    }

    pub fn set_transitions(&mut self, transitions: Vec<Transition>) {
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
