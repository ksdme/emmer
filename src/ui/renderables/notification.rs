use std::{
    ops::Div,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};

use crate::{
    config::{ComputedConfig, Insets},
    notification::Notification,
    ui::renderables::{Color, Rect, card::CardRenderable, text::TextRenderable},
};

/// Renders a notification card.
#[derive(Debug)]
pub struct NotificationRenderable {
    width: f64,
    padding: Insets,

    title_r: Option<TextRenderable>,
    body_r: Option<TextRenderable>,
    card_r: CardRenderable,
}

impl NotificationRenderable {
    pub fn new(config: &ComputedConfig, notification: &Notification) -> Self {
        // TODO: This should be a calculated value available here instead.
        let inner_w = (config.width - 2. * config.padding.x) as i32;
        Self {
            width: config.width,
            padding: config.padding.clone(),

            title_r: notification.title().map(|title| {
                TextRenderable::new(
                    &config.theme.font_map,
                    &config.theme.title_font_description,
                    Some(inner_w),
                    None,
                    title,
                )
            }),
            body_r: notification.body().map(|body| {
                TextRenderable::new(
                    &config.theme.font_map,
                    &config.theme.body_font_description,
                    Some(inner_w),
                    None,
                    body,
                )
            }),
            card_r: CardRenderable::new(),
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

/// Represents the visual style of the notification renderable.
#[derive(Clone, Default, Debug)]
pub struct Style {
    pub w: f64,
    pub h: f64,

    pub x: f64,
    pub y: f64,

    pub outer_opacity: f64,
    pub inner_opacity: f64,
}

impl From<Style> for PartialStyle {
    fn from(val: Style) -> Self {
        PartialStyle {
            w: Some(val.w),
            h: Some(val.h),

            x: Some(val.x),
            y: Some(val.y),

            inner_opacity: Some(val.inner_opacity),
            outer_opacity: Some(val.outer_opacity),
        }
    }
}

/// Represents a style object that can be used for property
/// based transitions.
#[derive(Debug, Default)]
pub struct PartialStyle {
    pub w: Option<f64>,
    pub h: Option<f64>,

    pub x: Option<f64>,
    pub y: Option<f64>,

    pub inner_opacity: Option<f64>,
    pub outer_opacity: Option<f64>,
}

/// Represents the parameters of a transition of a Style into another.
#[derive(Debug)]
pub struct Transition {
    start_at: Instant,
    duration: Duration,

    from: Option<Style>,
    to: PartialStyle,
}

macro_rules! interp {
    ($target:expr, $from:expr, $current:expr, $progress:expr) => {
        if let Some(target) = $target
            && target != $from
        {
            $from + (target - $from) * $progress
        } else {
            $current
        }
    };
}

impl Transition {
    /// Returns a new transition to target_state with a shared clock.
    pub fn new(duration: Duration, to: PartialStyle, start_at: Option<Instant>) -> Self {
        Self {
            start_at: start_at.unwrap_or_else(Instant::now),
            duration,

            from: None,
            to,
        }
    }

    /// Interpolates the transition to an Instant.
    pub fn interpolate(&mut self, current: &Style, now: &Instant) -> (Style, bool) {
        // The progress of the transition as [0, 1].
        let progress = now
            .checked_duration_since(self.start_at)
            .unwrap_or_default()
            .as_secs_f64()
            .div(self.duration.as_secs_f64())
            .clamp(0., 1.);

        // The first interpolation request is treated as the starting point of
        // the transition.
        let from = self.from.get_or_insert_with(|| current.clone());

        // Return an interpolated visual state along with a boolean indicating if the
        // transition is complete.
        let to = &self.to;
        (
            Style {
                w: interp!(to.w, from.w, current.w, progress),
                h: interp!(to.h, from.h, current.h, progress),
                x: interp!(to.x, from.x, current.x, progress),
                y: interp!(to.y, from.y, current.y, progress),
                outer_opacity: interp!(
                    to.outer_opacity,
                    from.outer_opacity,
                    current.outer_opacity,
                    progress
                ),
                inner_opacity: interp!(
                    to.inner_opacity,
                    from.inner_opacity,
                    current.inner_opacity,
                    progress
                ),
            },
            progress >= 1.,
        )
    }
}
