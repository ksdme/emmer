use anyhow::{Context, Result};

use crate::{
    config::{ButtonTheme, ComputedConfig},
    ui::renderables::{Rect, Renderable, Size, card, text},
};

/// Represents a generic button.
#[derive(Debug)]
pub struct Button {
    theme: ButtonTheme,

    label_r: text::Text,
}

impl Button {
    pub fn new(config: &ComputedConfig, label: &str, max_w: Option<i32>) -> Self {
        let theme = config.theme.button.clone();
        let padding = &theme.surface.padding;

        Self {
            label_r: text::Text::new(
                &config.font_map,
                &theme.label.font,
                max_w.map(|w| w - (2. * padding.x) as i32),
                Some(-1),
                label,
            ),

            theme,
        }
    }
}

impl Renderable for Button {
    type Style = Style;

    fn measure(&self) -> Size {
        let padding = &self.theme.surface.padding;
        let label = self.label_r.measure();
        Size {
            w: 2. * padding.x + label.w,
            h: 2. * padding.y + label.h,
        }
    }

    fn paint(&self, cr: &cairo::Context, rect: Rect, style: &Style) -> Result<()> {
        let surface = &self.theme.surface;

        let mut box_style = card::Style::from_surface(surface, style.opacity);
        box_style.bg = box_style.bg.with_lightness(style.light);
        card::paint(cr, rect, &box_style).context("Could not draw background")?;

        self.label_r
            .paint(
                cr,
                Rect::from_origin(
                    rect.x1 + surface.padding.x,
                    rect.y1 + surface.padding.y,
                    self.label_r.measure(),
                ),
                &text::Style {
                    color: self.theme.label.color.faded(style.opacity),
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
