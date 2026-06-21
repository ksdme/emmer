use anyhow::{Context, Result};

use crate::{
    config::{ComputedConfig, Insets},
    notification::Notification,
    ui::renderables::{Color, Rect, card, text},
};

/// Renders a notification card.
#[derive(Debug)]
pub struct Renderable {
    width: f64,
    padding: Insets,

    card_r: card::Renderable,
    title_r: Option<text::Renderable>,
    body_r: Option<text::Renderable>,
}

impl Renderable {
    pub fn new(config: &ComputedConfig, notification: &Notification) -> Self {
        // TODO: This should be a calculated value available here instead.
        let inner_w = (config.width - 2. * config.padding.x) as i32;
        Self {
            width: config.width,
            padding: config.padding.clone(),

            card_r: card::Renderable::new(),
            title_r: notification.title().map(|title| {
                text::Renderable::new(
                    &config.theme.font_map,
                    &config.theme.title_font_description,
                    Some(inner_w),
                    None,
                    title,
                )
            }),
            body_r: notification.body().map(|body| {
                text::Renderable::new(
                    &config.theme.font_map,
                    &config.theme.body_font_description,
                    Some(inner_w),
                    None,
                    body,
                )
            }),
        }
    }

    fn inner_height(&self) -> f64 {
        let t_h = self
            .title_r
            .as_ref()
            .map(|title| title.content_size())
            .map(|(_, h)| h)
            .unwrap_or_default();

        let b_h = self
            .body_r
            .as_ref()
            .map(|body| body.content_size())
            .map(|(_, h)| h)
            .unwrap_or_default();

        t_h + b_h + if t_h > 0. && b_h > 0. { 8. } else { 0. }
    }

    pub fn content_size(&self) -> (f64, f64) {
        (self.width, self.inner_height() + 2. * self.padding.y)
    }

    pub fn render(&self, cr: &cairo::Context, style: &Style) -> Result<Rect> {
        // The base.
        let rect = self
            .card_r
            .render(
                cr,
                style.x,
                style.y,
                style.w,
                style.h,
                Color::from_rgba_u8(52, 52, 52, style.outer_opacity),
                1.5,
                Some(Color::from_rgba_u8(102, 102, 102, style.outer_opacity)),
                8.,
            )
            .context("Could not draw block")?;

        let fg = Color::from_rgba_u8(255, 255, 255, style.inner_opacity);
        if style.inner_opacity > 0. {
            cr.save()
                .context("Could not save the current cairo state for clipping")?;

            let avail_w = rect.w() - 2. * self.padding.x;
            let avail_h = rect.h() - 2. * self.padding.y;
            let content_h = self.inner_height();

            cr.new_path();
            let content_x = rect.x1 + self.padding.x;
            let content_y = if content_h > avail_h {
                cr.rectangle(content_x, rect.y1 + self.padding.y, avail_w, avail_h);
                cr.clip();

                // Since the clip is a fixed window, the easiest way to anchor to bottom
                // is to do it during the draw.
                // TODO: Does bottom anchoring work in all cases?
                rect.y1 + self.padding.y - (content_h - avail_h)
            } else {
                rect.y1 + self.padding.y
            };

            // The title.
            let content_y = match &self.title_r {
                Some(title) => {
                    let (_, h) = title.render(cr, content_x, content_y, fg);
                    content_y + h + 8.
                }
                None => content_y,
            };

            // The body.
            let _content_y = match &self.body_r {
                Some(body) => {
                    let (_, h) = body.render(cr, content_x, content_y, fg);
                    content_y + h + 8.
                }
                None => content_y,
            };

            cr.restore()
                .context("Could not restore cairo state after clip")?;
        }

        Ok(rect)
    }
}

transitionable!(
    // Represents the box style and the contents style of the notification
    // renderable.
    Style {
        w: f64,
        h: f64,

        x: f64,
        y: f64,

        inner_opacity: f64,
        outer_opacity: f64,
    }
);
