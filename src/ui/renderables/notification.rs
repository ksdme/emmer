use anyhow::{Context, Result};

use crate::{
    config::{ComputedConfig, Insets},
    dbus, logged,
    ui::{
        anchors::VerticalAnchor,
        renderables::{Color, Rect, Renderable, Size, card, image, text},
    },
};

/// Renders a notification card.
#[derive(Debug)]
pub struct Notification {
    width: f64,
    padding: Insets,

    // The edge the contents stick to when they are clipped.
    clip_anchor: VerticalAnchor,

    image_r: Option<image::Image>,
    title_r: Option<text::Text>,
    body_r: Option<text::Text>,

    app_r: Option<text::Text>,
    created_at_r: Option<text::Text>,
}

impl Notification {
    pub fn new(
        config: &ComputedConfig,
        app: Option<&str>,
        image: Option<dbus::image::Image>,
        title: Option<&str>,
        body: Option<&str>,
        created_at: Option<&str>,
    ) -> Result<Self> {
        let image_r = if let Some(image) = image {
            logged!(image::Image::new(image).context("Could not initialize image")).ok()
        } else {
            None
        };

        let inner_w = (config.width
            - (2. + if image_r.is_some() { 1. } else { 0. }) * config.padding.x
            - image_r.as_ref().map(|r| r.measure().w).unwrap_or(0.)) as i32;

        Ok(Self {
            width: config.width,
            padding: config.padding.clone(),

            clip_anchor: VerticalAnchor::Top,

            image_r,
            title_r: title.map(|title| {
                text::Text::new(
                    &config.theme.font_map,
                    &config.theme.title_font_description,
                    Some(inner_w),
                    None,
                    title,
                )
            }),
            body_r: body.map(|body| {
                text::Text::new(
                    &config.theme.font_map,
                    &config.theme.body_font_description,
                    Some(inner_w),
                    None,
                    body,
                )
            }),

            app_r: app.map(|app| {
                text::Text::new(
                    &config.theme.font_map,
                    &config.theme.body_font_description,
                    None,
                    Some(-1),
                    app,
                )
            }),
            created_at_r: created_at.map(|created| {
                text::Text::new(
                    &config.theme.font_map,
                    &config.theme.body_font_description,
                    None,
                    Some(-1),
                    created,
                )
            }),
        })
    }

    // Sets the edge the contents stick to when the card is shorter than them.
    pub fn set_clip_anchor(&mut self, clip_anchor: VerticalAnchor) {
        self.clip_anchor = clip_anchor;
    }

    // Height of the core contents.
    fn content_height(&self) -> f64 {
        let title_h = self.title_r.as_ref().map(|r| r.measure().h).unwrap_or(0.);
        let body_h = self.body_r.as_ref().map(|r| r.measure().h).unwrap_or(0.);
        let image_h = self.image_r.as_ref().map(|r| r.measure().h).unwrap_or(0.);

        image_h.max(title_h + body_h + if title_h > 0. && body_h > 0. { 8. } else { 0. })
    }

    // Height contributed by the non core contents.
    fn meta_height(&self) -> f64 {
        let name_h = self.app_r.as_ref().map(|r| r.measure().h).unwrap_or(0.);
        let created_h = self
            .created_at_r
            .as_ref()
            .map(|r| r.measure().h)
            .unwrap_or(0.);

        name_h.max(created_h)
    }

    // The total height of the non padding contents.
    fn inner_height(&self) -> f64 {
        let m_h = self.meta_height();
        let c_h = self.content_height();
        c_h + m_h + if c_h > 0. && m_h > 0. { 6. } else { 0. }
    }
}

impl Renderable for Notification {
    type Style = Style;

    fn measure(&self) -> Size {
        Size {
            w: self.width,
            h: self.inner_height() + 2. * self.padding.y,
        }
    }

    fn paint(&self, cr: &cairo::Context, rect: Rect, style: &Style) -> Result<()> {
        // The base.
        card::paint(
            cr,
            rect,
            &card::Style {
                bg: Color::from_rgba_u8(52, 52, 52, style.outer_opacity),
                border_width: 1.5,
                border_color: Color::from_rgba_u8(102, 102, 102, style.outer_opacity),
                border_radius: 8.,
            },
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
                    + match self.clip_anchor {
                        VerticalAnchor::Top => 0.,
                        VerticalAnchor::Bottom => -(content_h - avail_h),
                    }
            } else {
                rect.y1 + self.padding.y
            };

            // The image/icon.
            let content_x = if let Some(image_r) = self.image_r.as_ref() {
                let size = image_r.measure();
                match logged!(
                    image_r
                        .paint(
                            cr,
                            Rect::from_origin(content_x, content_y, size),
                            &image::Style {
                                opacity: style.inner_opacity,
                            },
                        )
                        .context("Could not draw image")
                ) {
                    Ok(()) => content_x + size.w + 8.,
                    Err(_) => content_x,
                }
            } else {
                content_x
            };

            cr.new_path();
            let fg = text::Style {
                color: Color::from_rgba_u8(255, 255, 255, style.inner_opacity),
            };
            let meta = text::Style {
                color: fg.color.with_alpha(style.inner_opacity * 0.35),
            };

            // The title.
            let content_y = match &self.title_r {
                Some(title) => {
                    let size = title.measure();
                    title.paint(cr, Rect::from_origin(content_x, content_y, size), &fg)?;
                    content_y + size.h + 8.
                }
                None => content_y,
            };

            // The body.
            if let Some(body) = &self.body_r {
                body.paint(
                    cr,
                    Rect::from_origin(content_x, content_y, body.measure()),
                    &fg,
                )?;
            }

            // The footer.
            if let Some(app_r) = &self.app_r {
                let size = app_r.measure();
                app_r.paint(
                    cr,
                    Rect::from_origin(
                        rect.x1 + self.padding.x,
                        rect.y2 - self.padding.y - size.h,
                        size,
                    ),
                    &meta,
                )?;
            }

            if let Some(created_at_r) = &self.created_at_r {
                let size = created_at_r.measure();
                created_at_r.paint(
                    cr,
                    Rect::from_origin(
                        rect.x2 - self.padding.x - size.w,
                        rect.y2 - self.padding.y - size.h,
                        size,
                    ),
                    &meta,
                )?;
            }

            cr.restore()
                .context("Could not restore cairo state after clip")?;
        }

        Ok(())
    }
}

transitionable!(
    // Represents the box style and the contents style of the notification
    // renderable.
    Style {
        inner_opacity: f64,
        outer_opacity: f64,
    }
);
