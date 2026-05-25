use std::time::Instant;

use skia_safe::{
    ClipOp, Rect,
    textlayout::{Paragraph, ParagraphBuilder, ParagraphStyle},
};

use crate::{
    config::ComputedConfig,
    notification::Notification,
    ui::{
        draw::block,
        items::style::{Style, Transition},
    },
};

#[derive(Debug, PartialEq)]
pub enum State {
    Alive,
    Dismissed,
}

#[derive(Debug)]
pub struct Item {
    pub state: State,

    notification: Notification,
    render_cache: ItemRenderCache,

    style: Style,
    transitions: Vec<Transition>,

    bounds: Option<Rect>,
}

impl Item {
    pub fn new(config: &ComputedConfig, notification: Notification) -> Self {
        Self {
            state: State::Alive,

            render_cache: ItemRenderCache::new(config, &notification),
            notification,

            style: Style::default(),
            transitions: vec![],

            bounds: None,
        }
    }

    pub fn id(&self) -> u32 {
        self.notification.id
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

    /// Renders the current item to a skia canvas and returns its rect bounds.
    pub fn render(&mut self, config: &ComputedConfig, canvas: &skia_safe::Canvas) -> &Rect {
        let rect = Rect::from_xywh(self.style.x, self.style.y, self.style.w, self.style.h);
        block::draw_block(
            &block::Block {
                shadow: Some(block::Shadow::SM),
                ..Default::default()
            },
            canvas,
            &rect,
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

        self.bounds = Some(rect);
        self.bounds.as_ref().unwrap()
    }

    pub fn set_style(&mut self, style: Style) {
        self.style = style;
    }

    pub fn set_transitions(&mut self, transitions: Vec<Transition>) {
        self.transitions = transitions;
    }

    pub fn hitbox(&self) -> Option<&Rect> {
        if self.style.visible() {
            self.bounds.as_ref()
        } else {
            None
        }
    }

    pub fn content_size(&self, config: &ComputedConfig) -> (f32, f32) {
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
    pub fn new(config: &ComputedConfig, notification: &Notification) -> Self {
        let w = config.width - 2. * config.padding.x;

        let mut p_style = ParagraphStyle::default();
        p_style.set_max_lines(4);

        let mut title_p = ParagraphBuilder::new(&p_style, config.theme.font_collection.clone())
            .push_style(&config.theme.title_style)
            .add_text(&notification.summary)
            .build();
        title_p.layout(w);

        let mut body_p = ParagraphBuilder::new(&p_style, config.theme.font_collection.clone())
            .push_style(&config.theme.body_style)
            .add_text(&notification.body)
            .build();
        body_p.layout(w);

        Self { title_p, body_p }
    }

    pub fn content_height(&self) -> f32 {
        self.title_p.height() + self.body_p.height() + 8.
    }
}
