use std::rc::Rc;

use anyhow::{Context, Result};

use crate::{
    config::{self, ComputedConfig},
    dbus, logged,
    ui::{
        anchors::VerticalAnchor,
        renderables::{Rect, Renderable, Size, card, image, text},
    },
};

/// Renders a notification card.
#[derive(Debug)]
pub struct Notification {
    config: Rc<ComputedConfig>,

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
        config: Rc<ComputedConfig>,
        app: Option<&str>,
        image: Option<dbus::image::Image>,
        title: Option<&str>,
        body: Option<&str>,
        created_at: Option<&str>,
    ) -> Result<Self> {
        let theme = &config.theme;
        let padding = &theme.notification.padding;

        let image_r = if let Some(image) = image {
            logged!(
                image::Image::new(image, theme.image_radius).context("Could not initialize image")
            )
            .ok()
        } else {
            None
        };

        let inner_w = (config.width
            - 2. * padding.x
            - image_r
                .as_ref()
                .map(|r| r.measure().w + theme.image_gap)
                .unwrap_or(0.)) as i32;

        let text = |font: &pango::FontDescription, max_w, max_h, content: &str| {
            text::Text::new(&config.font_map, font, max_w, max_h, content)
        };

        Ok(Self {
            clip_anchor: VerticalAnchor::Top,

            image_r,
            title_r: title.map(|title| text(&theme.title.font, Some(inner_w), None, title)),
            body_r: body.map(|body| text(&theme.body.font, Some(inner_w), None, body)),

            app_r: app.map(|app| text(&theme.footer.font, None, Some(-1), app)),
            created_at_r: created_at
                .map(|created| text(&theme.footer.font, None, Some(-1), created)),

            config,
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

        let title_gap = if title_h > 0. && body_h > 0. {
            self.config.theme.title_gap
        } else {
            0.
        };

        image_h.max(title_h + body_h + title_gap)
    }

    // Height contributed by the non core contents.
    fn footer_height(&self) -> f64 {
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
        let m_h = self.footer_height();
        let c_h = self.content_height();
        let footer_gap = if c_h > 0. && m_h > 0. {
            self.config.theme.footer_gap
        } else {
            0.
        };

        c_h + m_h + footer_gap
    }
}

impl Renderable for Notification {
    type Style = Style;

    fn measure(&self) -> Size {
        Size {
            w: self.config.width,
            h: self.inner_height() + 2. * self.config.theme.notification.padding.y,
        }
    }

    fn paint(&self, cr: &cairo::Context, rect: Rect, style: &Style) -> Result<()> {
        let theme = &self.config.theme;
        let padding = &theme.notification.padding;

        // The base.
        card::paint(
            cr,
            rect,
            &card::Style::from_surface(&theme.notification, style.outer_opacity),
        )
        .context("Could not draw block")?;

        if style.inner_opacity > 0. {
            cr.save()
                .context("Could not save the current cairo state for clipping")?;

            let content_h = self.inner_height();
            let avail_w = rect.w() - 2. * padding.x;
            let avail_h = rect.h() - 2. * padding.y;

            let content_x = rect.x1 + padding.x;
            let content_y = if content_h > avail_h {
                // Clip the extra content.
                cr.rectangle(content_x, rect.y1 + padding.y, avail_w, avail_h);
                cr.clip();

                // Since the clip is a fixed window, the easiest way to anchor content is to
                // do it using an offset.
                rect.y1
                    + padding.y
                    + match self.clip_anchor {
                        VerticalAnchor::Top => 0.,
                        VerticalAnchor::Bottom => -(content_h - avail_h),
                    }
            } else {
                rect.y1 + padding.y
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
                    Ok(()) => content_x + size.w + theme.image_gap,
                    Err(_) => content_x,
                }
            } else {
                content_x
            };

            cr.new_path();
            let text_style = |text: &config::Text| text::Style {
                color: text.color.faded(style.inner_opacity),
            };

            // The title.
            let content_y = match &self.title_r {
                Some(title) => {
                    let size = title.measure();
                    title.paint(
                        cr,
                        Rect::from_origin(content_x, content_y, size),
                        &text_style(&theme.title),
                    )?;
                    content_y + size.h + theme.title_gap
                }
                None => content_y,
            };

            // The body.
            if let Some(body) = &self.body_r {
                body.paint(
                    cr,
                    Rect::from_origin(content_x, content_y, body.measure()),
                    &text_style(&theme.body),
                )?;
            }

            // The footer.
            if let Some(app_r) = &self.app_r {
                let size = app_r.measure();
                app_r.paint(
                    cr,
                    Rect::from_origin(rect.x1 + padding.x, rect.y2 - padding.y - size.h, size),
                    &text_style(&theme.footer),
                )?;
            }

            if let Some(created_at_r) = &self.created_at_r {
                let size = created_at_r.measure();
                created_at_r.paint(
                    cr,
                    Rect::from_origin(
                        rect.x2 - padding.x - size.w,
                        rect.y2 - padding.y - size.h,
                        size,
                    ),
                    &text_style(&theme.footer),
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
