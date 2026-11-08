use anyhow::{Context, Result};

use crate::{
    config::{ComputedConfig, Insets},
    ui::renderables::{Color, Rect, Renderable, Size, card, rounded_sub_path, text},
};

const RADIUS: f64 = 4.;

/// Renders the dismiss-all bar, which shows the count of the notifications and
/// a control to clear all of them.
#[derive(Debug)]
pub struct Bar {
    width: f64,
    padding: Insets,

    count_r: text::Text,
    clear_r: text::Text,
}

impl Bar {
    pub fn new(config: &ComputedConfig, count: usize, padding: Insets) -> Self {
        let text = |label: &str| {
            text::Text::new(
                &config.theme.font_map,
                &config.theme.body_font_description,
                None,
                Some(-1),
                label,
            )
        };

        Self {
            width: config.width,
            padding,

            count_r: text(&format!("{count} Notifications")),
            clear_r: text("Clear"),
        }
    }

    /// Returns the rect the clear control occupies when the bar is painted
    /// into rect.
    pub fn clear_rect(&self, rect: Rect) -> Rect {
        Rect {
            x1: rect.x2 - 2. * self.padding.x - self.clear_r.measure().w,
            ..rect
        }
    }
}

impl Renderable for Bar {
    type Style = Style;

    fn measure(&self) -> Size {
        let count = self.count_r.measure();
        let clear = self.clear_r.measure();
        Size {
            w: self.width,
            h: 2. * self.padding.y + count.h.max(clear.h),
        }
    }

    fn paint(&self, cr: &cairo::Context, rect: Rect, style: &Style) -> Result<()> {
        let border = Color::from_rgba_u8(102, 102, 102, style.opacity);
        let fg = text::Style {
            color: Color::from_rgba_u8(255, 255, 255, style.opacity),
        };

        card::paint(
            cr,
            rect,
            &card::Style {
                bg: Color::from_rgba_u8(52, 52, 52, style.opacity),
                border_width: 1.,
                border_color: border,
                border_radius: RADIUS,
            },
        )
        .context("Could not draw box")?;

        // The hover tint on the clear control, which is clipped to the rounded
        // shape of the bar.
        let clear = self.clear_rect(rect);
        if style.clear_light > 0. {
            // TODO: This is not the ideal way of doing this.
            cr.save().context("Could not save the cairo state")?;
            cr.new_path();
            rounded_sub_path(cr, rect.x1, rect.y1, rect.w(), rect.h(), RADIUS);
            cr.clip();

            cr.rectangle(clear.x1, clear.y1, clear.w(), clear.h());
            cr.set_source_rgba(1., 1., 1., style.clear_light * style.opacity);
            cr.fill().context("Could not draw clear tint")?;

            cr.restore().context("Could not restore the cairo state")?;
        }

        // The divider.
        cr.new_path();
        cr.move_to(clear.x1, rect.y1);
        cr.line_to(clear.x1, rect.y2);
        cr.set_source_rgba(border.r, border.g, border.b, border.a);
        cr.set_line_width(0.5);
        cr.stroke().context("Could not draw divider")?;

        let count = self.count_r.measure();
        self.count_r
            .paint(
                cr,
                Rect::from_origin(
                    rect.x1 + self.padding.x,
                    rect.y1 + (rect.h() - count.h) / 2.,
                    count,
                ),
                &fg,
            )
            .context("Could not draw count")?;

        let size = self.clear_r.measure();
        self.clear_r
            .paint(
                cr,
                Rect::from_origin(
                    clear.x1 + self.padding.x,
                    rect.y1 + (rect.h() - size.h) / 2.,
                    size,
                ),
                &fg,
            )
            .context("Could not draw clear")
    }
}

/// The style of the bar renderable.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Style {
    pub clear_light: f64,
    pub opacity: f64,
}
