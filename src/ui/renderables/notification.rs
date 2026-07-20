use anyhow::{Context, Result};

use crate::{
    config::{ComputedConfig, Insets},
    dbus::notification,
    logged,
    ui::{
        anchors::VerticalAnchor,
        renderables::{Color, Rect, card, image, text},
    },
};

/// Renders a notification card.
#[derive(Debug)]
pub struct Renderable {
    width: f64,
    padding: Insets,

    card_r: card::Renderable,

    image_r: Option<image::Renderable>,
    title_r: Option<text::Renderable>,
    body_r: Option<text::Renderable>,

    app_r: Option<text::Renderable>,
    created_at_r: Option<text::Renderable>,
}

impl Renderable {
    pub fn new(
        config: &ComputedConfig,
        app: Option<&str>,
        image: Option<notification::ImageSource>,
        title: Option<&str>,
        body: Option<&str>,
        created_at: Option<&str>,
    ) -> Result<Self> {
        let image_r = if let Some(image_source) = image {
            logged!(
                image::Renderable::from_source(image_source, 64., Some(64.))
                    .context("Could not initialize image")
            )
            .ok()
        } else {
            None
        };

        let inner_w = (config.width
            - (2. + if image_r.is_some() { 1. } else { 0. }) * config.padding.x
            - image_r
                .as_ref()
                .map(|r| r.content_size())
                .map(|(w, _)| w)
                .unwrap_or(0.)) as i32;

        Ok(Self {
            width: config.width,
            padding: config.padding.clone(),

            card_r: card::Renderable::new(),

            image_r,
            title_r: title.map(|title| {
                text::Renderable::new(
                    &config.theme.font_map,
                    &config.theme.title_font_description,
                    Some(inner_w),
                    None,
                    title,
                )
            }),
            body_r: body.map(|body| {
                text::Renderable::new(
                    &config.theme.font_map,
                    &config.theme.body_font_description,
                    Some(inner_w),
                    None,
                    body,
                )
            }),

            app_r: app.map(|app| {
                text::Renderable::new(
                    &config.theme.font_map,
                    &config.theme.body_font_description,
                    None,
                    Some(-1),
                    app,
                )
            }),
            created_at_r: created_at.map(|created| {
                text::Renderable::new(
                    &config.theme.font_map,
                    &config.theme.body_font_description,
                    None,
                    Some(-1),
                    created,
                )
            }),
        })
    }

    // Height of the core contents.
    fn content_height(&self) -> f64 {
        let title_h = self
            .title_r
            .as_ref()
            .map(|title| title.content_size())
            .map(|(_, h)| h)
            .unwrap_or(0.);

        let body_h = self
            .body_r
            .as_ref()
            .map(|body| body.content_size())
            .map(|(_, h)| h)
            .unwrap_or(0.);

        let image_h = self
            .image_r
            .as_ref()
            .map(|image| image.content_size())
            .map(|(_, h)| h)
            .unwrap_or(0.);

        image_h.max(title_h + body_h + if title_h > 0. && body_h > 0. { 8. } else { 0. })
    }

    // Height contributed by the non core contents.
    fn meta_height(&self) -> f64 {
        let name_h = self
            .app_r
            .as_ref()
            .map(|r| r.content_size())
            .map(|(_, h)| h)
            .unwrap_or(0.);

        let created_h = self
            .created_at_r
            .as_ref()
            .map(|r| r.content_size())
            .map(|(_, h)| h)
            .unwrap_or(0.);

        name_h.max(created_h)
    }

    // The total height of the non padding contents.
    fn inner_height(&self) -> f64 {
        let m_h = self.meta_height();
        let c_h = self.content_height();
        c_h + m_h + if c_h > 0. && m_h > 0. { 6. } else { 0. }
    }

    pub fn content_size(&self) -> (f64, f64) {
        (self.width, self.inner_height() + 2. * self.padding.y)
    }

    pub fn render(
        &self,
        cr: &cairo::Context,
        style: &Style,
        clip_anchor: VerticalAnchor,
    ) -> Result<Rect> {
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

        if style.inner_opacity > 0. {
            cr.save()
                .context("Could not save the current cairo state for clipping")?;

            let content_h = self.inner_height();
            let avail_w = rect.w() - 2. * self.padding.x;
            let avail_h = rect.h() - 2. * self.padding.y;

            let content_x = rect.x1 + self.padding.x;
            let content_y = if content_h > avail_h {
                // Clip the extra content.
                cr.rectangle(content_x, rect.y1 + self.padding.y, avail_w, avail_h);
                cr.clip();

                // Since the clip is a fixed window, the easiest way to anchor content is to
                // do it using an offset.
                rect.y1
                    + self.padding.y
                    + match clip_anchor {
                        VerticalAnchor::Top => 0.,
                        VerticalAnchor::Bottom => -(content_h - avail_h),
                    }
            } else {
                rect.y1 + self.padding.y
            };

            // The image/icon.
            let content_x = if let Some(image_r) = self.image_r.as_ref() {
                match logged!(
                    image_r
                        .render(cr, content_x, content_y, style.inner_opacity)
                        .context("Could not draw image")
                ) {
                    Ok(bounds) => content_x + bounds.w() + 8.,
                    Err(_) => content_x,
                }
            } else {
                content_x
            };

            cr.new_path();
            let fg = Color::from_rgba_u8(255, 255, 255, style.inner_opacity);

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
                    content_y + h + 4.
                }
                None => content_y,
            };

            // The footer.
            if let Some(app_r) = &self.app_r {
                let (_, h) = app_r.content_size();
                app_r.render(
                    cr,
                    rect.x1 + self.padding.x,
                    rect.y2 - self.padding.y - h,
                    fg.with_alpha(style.inner_opacity * 0.35),
                );
            }

            if let Some(created_at_r) = &self.created_at_r {
                let (w, h) = created_at_r.content_size();
                created_at_r.render(
                    cr,
                    rect.x2 - self.padding.x - w,
                    rect.y2 - self.padding.y - h,
                    fg.with_alpha(style.inner_opacity * 0.35),
                );
            }

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
