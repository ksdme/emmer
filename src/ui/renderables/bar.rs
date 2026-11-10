use std::rc::Rc;

use anyhow::{Context, Result};

use crate::{
    config::ComputedConfig,
    ui::renderables::{Rect, Renderable, Size, card, rounded_sub_path, text},
};

/// Renders the dismiss-all bar, which shows the count of the notifications and
/// a control to clear all of them.
#[derive(Debug)]
pub struct Bar {
    config: Rc<ComputedConfig>,

    count_r: text::Text,
    clear_r: text::Text,
}

impl Bar {
    pub fn new(config: Rc<ComputedConfig>, count: usize) -> Self {
        let font = &config.theme.bar_label.font;
        let text = |label: &str| text::Text::new(&config.font_map, font, None, Some(-1), label);

        Self {
            count_r: text(&format!("{count} Notifications")),
            clear_r: text("Clear"),

            config,
        }
    }

    /// Returns the rect the clear control occupies when the bar is painted
    /// into rect.
    pub fn clear_rect(&self, rect: Rect) -> Rect {
        Rect {
            x1: rect.x2 - 2. * self.config.theme.bar.padding.x - self.clear_r.measure().w,
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
            w: self.config.width,
            h: 2. * self.config.theme.bar.padding.y + count.h.max(clear.h),
        }
    }

    fn paint(&self, cr: &cairo::Context, rect: Rect, style: &Style) -> Result<()> {
        let theme = &self.config.theme;
        let padding = &theme.bar.padding;
        let box_style = card::Style::from_surface(&theme.bar, style.opacity);
        let border = box_style.border_color;
        let fg = text::Style {
            color: theme.bar_label.color.faded(style.opacity),
        };

        card::paint(cr, rect, &box_style).context("Could not draw box")?;

        // The hover tint on the clear control, which is clipped to the rounded
        // shape of the bar.
        let clear = self.clear_rect(rect);
        if style.clear_light > 0. {
            // TODO: This is not the ideal way of doing this.
            cr.save().context("Could not save the cairo state")?;
            cr.new_path();
            rounded_sub_path(
                cr,
                rect.x1,
                rect.y1,
                rect.w(),
                rect.h(),
                box_style.border_radius,
            );
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
                    rect.x1 + padding.x,
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
                    clear.x1 + padding.x,
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
