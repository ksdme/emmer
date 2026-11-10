use std::rc::Rc;

use anyhow::{Context, Result};

use crate::{
    config::ComputedConfig,
    ui::renderables::{Rect, Renderable, Size, card, text},
};

/// Represents a generic button.
#[derive(Debug)]
pub struct Button {
    config: Rc<ComputedConfig>,

    label_r: text::Text,
}

impl Button {
    pub fn new(config: Rc<ComputedConfig>, label: &str, max_w: Option<i32>) -> Self {
        let theme = &config.theme;
        let padding = &theme.button.padding;

        Self {
            label_r: text::Text::new(
                &config.font_map,
                &theme.button_label.font,
                max_w.map(|w| w - (2. * padding.x) as i32),
                Some(-1),
                label,
            ),

            config,
        }
    }
}

impl Renderable for Button {
    type Style = Style;

    fn measure(&self) -> Size {
        let padding = &self.config.theme.button.padding;
        let label = self.label_r.measure();
        Size {
            w: 2. * padding.x + label.w,
            h: 2. * padding.y + label.h,
        }
    }

    fn paint(&self, cr: &cairo::Context, rect: Rect, style: &Style) -> Result<()> {
        let theme = &self.config.theme;
        let surface = &theme.button;

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
                    color: theme.button_label.color.faded(style.opacity),
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
