use anyhow::{Context, Result};

use crate::{
    config::{ComputedConfig, Insets},
    ui::renderables::{Color, Rect, card, text},
};

/// Represents a generic button.
#[derive(Debug)]
pub struct Renderable {
    padding: Insets,

    card_r: card::Renderable,
    label_r: text::Renderable,
}

impl Renderable {
    pub fn new(config: &ComputedConfig, label: &str, padding: Insets, max_w: Option<i32>) -> Self {
        Self {
            card_r: card::Renderable::new(),
            label_r: text::Renderable::new(
                &config.theme.font_map,
                &config.theme.body_font_description,
                max_w.map(|w| w - (2. * padding.x) as i32),
                Some(-1),
                label,
            ),

            padding,
        }
    }

    pub fn content_size(&self) -> (f64, f64) {
        let (label_w, label_h) = self.label_r.content_size();
        (2. * self.padding.x + label_w, 2. * self.padding.y + label_h)
    }

    pub fn render(&self, cr: &cairo::Context, style: &Style, x: f64, y: f64) -> Result<Rect> {
        let (w, h) = self.content_size();

        let bounds = self
            .card_r
            .render(
                cr,
                x,
                y,
                w,
                h,
                Color::from_rgba_u8(52, 52, 52, style.opacity).with_lightness(style.light),
                1.,
                Some(Color::from_rgba_u8(102, 102, 102, style.opacity)),
                4.,
            )
            .context("Could not draw background")?;

        let _ = self.label_r.render(
            cr,
            x + self.padding.x,
            y + self.padding.y,
            Color::from_rgba_u8(255, 255, 255, style.opacity),
        );

        Ok(bounds)
    }
}

transitionable!(
    // The style of a button renderable.
    Style {
        light: f64,
        opacity: f64,
    }
);
