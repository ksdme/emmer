use anyhow::{Context, Result};

use crate::{
    config::{ComputedConfig, Insets},
    ui::renderables::{Color, Rect, Renderable, Size, card, text},
};

/// Represents a generic button.
#[derive(Debug)]
pub struct Button {
    padding: Insets,

    label_r: text::Text,
}

impl Button {
    pub fn new(config: &ComputedConfig, label: &str, padding: Insets, max_w: Option<i32>) -> Self {
        Self {
            label_r: text::Text::new(
                &config.theme.font_map,
                &config.theme.body_font_description,
                max_w.map(|w| w - (2. * padding.x) as i32),
                Some(-1),
                label,
            ),

            padding,
        }
    }
}

impl Renderable for Button {
    type Style = Style;

    fn measure(&self) -> Size {
        let label = self.label_r.measure();
        Size {
            w: 2. * self.padding.x + label.w,
            h: 2. * self.padding.y + label.h,
        }
    }

    fn paint(&self, cr: &cairo::Context, rect: Rect, style: &Style) -> Result<()> {
        card::paint(
            cr,
            rect,
            &card::Style {
                bg: Color::from_rgba_u8(52, 52, 52, style.opacity).with_lightness(style.light),
                border_width: 1.,
                border_color: Color::from_rgba_u8(102, 102, 102, style.opacity),
                border_radius: 4.,
            },
        )
        .context("Could not draw background")?;

        self.label_r
            .paint(
                cr,
                Rect::from_origin(
                    rect.x1 + self.padding.x,
                    rect.y1 + self.padding.y,
                    self.label_r.measure(),
                ),
                &text::Style {
                    color: Color::from_rgba_u8(255, 255, 255, style.opacity),
                },
            )
            .context("Could not draw label")
    }
}

/// The style of a button renderable.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Style {
    pub light: f64,
    pub opacity: f64,
}
