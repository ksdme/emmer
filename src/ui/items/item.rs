use std::{f64::consts::PI, time::Instant};

use anyhow::{Context, Result};

use crate::{
    config::ComputedConfig,
    notification::Notification,
    ui::{
        bounds::Rect,
        colors::Color,
        items::style::{Style, Transition},
    },
};

#[derive(Debug, Clone, PartialEq)]
pub enum State {
    Alive,
    Dismissed,
}

#[derive(Debug)]
pub struct Item {
    state: State,

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
        self.notification.id()
    }

    pub fn notification(&self) -> &Notification {
        &self.notification
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

    /// Renders the current item to a cairo canvas and returns its rect bounds.
    pub fn render(&mut self, config: &ComputedConfig, cx: &cairo::Context) -> Result<&Rect> {
        let (x, y) = (self.style.x, self.style.y);
        let (w, h) = (self.style.w, self.style.h);

        let r = Some(8.);
        let bg = Color::from_rgba_u8(52, 52, 52, self.style.box_opacity);
        let fg = Color::from_rgba_u8(255, 255, 255, self.style.text_opacity);
        let border = Some((
            1.5,
            (Color::from_rgba_u8(102, 102, 102, self.style.box_opacity)),
        ));

        // Path of the block.
        cx.new_path();
        if let Some(r) = r {
            cx.new_sub_path();
            cx.arc(x + r, y + r, r, PI, 3.0 * PI / 2.0);
            cx.arc(x + w - r, y + r, r, 3.0 * PI / 2.0, 2.0 * PI);
            cx.arc(x + w - r, y + h - r, r, 0.0, PI / 2.0);
            cx.arc(x + r, y + h - r, r, PI / 2.0, PI);
            cx.close_path();
        } else {
            cx.rectangle(x, y, w, h);
        }

        // Background.
        cx.set_source_rgba(bg.r, bg.g, bg.b, bg.a);
        cx.fill_preserve()
            .context("Could not fill shape on main surface")?;

        // Add the border.
        if let Some((width, color)) = border {
            cx.set_line_width(width);
            cx.set_source_rgba(color.r, color.g, color.b, color.a);
            cx.stroke_preserve()
                .context("Could not stroke main surface path")?;
        }

        if self.style.text_opacity > 0. {
            cx.save()
                .context("Could not save the current cairo state for clipping")?;

            let avail_w = w - 2. * config.padding.x;
            let avail_h = h - 2. * config.padding.y;
            let content_h = self.content_height();

            cx.new_path();
            let content_x = x + config.padding.x;
            let content_y = if content_h > avail_h {
                cx.rectangle(content_x, y + config.padding.y, avail_w, avail_h);
                cx.clip();

                // Since the clip is a fixed window, the easiest way to anchor to bottom
                // is to do it during the draw.
                // TODO: Does bottom anchoring work in all cases?
                y + config.padding.y - (content_h - avail_h)
            } else {
                y + config.padding.y
            };

            // The title.
            cx.set_source_rgba(fg.r, fg.g, fg.b, fg.a);
            let content_y = match &self.render_cache.title {
                Some((title, h)) => {
                    cx.move_to(content_x, content_y);

                    pangocairo::functions::show_layout(cx, title);
                    content_y + h + 8.
                }
                None => content_y,
            };

            // The body.
            let _content_y = match &self.render_cache.body {
                Some((body, h)) => {
                    cx.move_to(content_x, content_y);

                    pangocairo::functions::show_layout(cx, body);
                    content_y + h + 8.
                }
                None => content_y,
            };

            cx.restore()
                .context("Could not restore cairo state after clip")?;
        }

        self.bounds = Some(Rect::from_xywh(x, y, w, h));
        Ok(self.bounds.as_ref().unwrap())
    }

    pub fn set_style(&mut self, style: Style) {
        self.style = style;
    }

    pub fn set_transitions(&mut self, transitions: Vec<Transition>) {
        self.transitions = transitions;
    }

    pub fn state(&self) -> State {
        self.state.clone()
    }

    pub fn set_state(&mut self, state: State) {
        self.state = state;
    }

    fn content_height(&self) -> f64 {
        let title_h = match &self.render_cache.title {
            Some((_, h)) => *h,
            None => 0.,
        };

        let body_h = match &self.render_cache.body {
            Some((_, h)) => *h,
            None => 0.,
        };

        title_h
            + body_h
            + if title_h > 0. && body_h > 0. {
                8. // The padding if both the lines of text exist.
            } else {
                0.
            }
    }

    pub fn content_size(&self, config: &ComputedConfig) -> (f64, f64) {
        (
            config.width,
            (2. * config.padding.y) + self.content_height(),
        )
    }

    pub fn hitbox(&self) -> Option<&Rect> {
        if self.style.visible() {
            self.bounds.as_ref()
        } else {
            None
        }
    }
}

#[derive(Debug)]
struct ItemRenderCache {
    title: Option<(pango::Layout, f64)>,
    body: Option<(pango::Layout, f64)>,
}

impl ItemRenderCache {
    pub fn new(config: &ComputedConfig, notification: &Notification) -> Self {
        // TODO: This should be a calculated value available here instead.
        let w = (config.width - 2. * config.padding.x) as i32;

        let cx = pango::Context::new();
        cx.set_font_map(Some(&config.theme.font_map));

        let title = notification.title().map(|title| {
            let layout = pango::Layout::new(&cx);

            layout.set_width(w * pango::SCALE);
            layout.set_wrap(pango::WrapMode::Word);

            layout.set_font_description(Some(&config.theme.title_font_description));
            layout.set_text(title);

            let (_, h) = layout.pixel_size();
            (layout, h as f64)
        });

        let body = notification.body().map(|body| {
            let layout = pango::Layout::new(&cx);

            layout.set_width(w * pango::SCALE);
            layout.set_wrap(pango::WrapMode::Word);

            layout.set_font_description(Some(&config.theme.body_font_description));
            layout.set_text(body);

            let (_, h) = layout.pixel_size();
            (layout, h as f64)
        });

        Self { title, body }
    }
}
