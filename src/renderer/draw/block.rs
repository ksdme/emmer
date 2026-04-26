use skia_safe::{
    Canvas, Color, Paint, PaintStyle, PathBuilder, PathDirection, RRect, Rect,
    utils::shadow_utils::ShadowFlags,
};

use crate::renderer::colors;

/// Represents a border configuration.
#[derive(Debug, Clone)]
pub struct Border {
    pub color: Color,
    pub width: f32,
}

/// Represents a shadow on the card.
#[derive(Debug, Clone)]
pub enum Shadow {
    XS,
    SM,
}

/// Represents a blank rounded card.
#[derive(Debug, Clone)]
pub struct Block {
    pub bg: Color,
    pub border: Option<Border>,
    pub radius: Option<f32>,
    pub shadow: Option<Shadow>,
}

impl Default for Block {
    fn default() -> Self {
        Self {
            bg: Color::from_rgb(52, 52, 52),
            border: Some(Border {
                color: Color::from_rgb(102, 102, 102),
                width: 2.5,
            }),
            radius: Some(8.0),
            shadow: None,
        }
    }
}

pub fn block(canvas: &Canvas, config: &Block, x: f32, y: f32, w: f32, h: f32, opacity: f32) {
    // The shape of the block.
    let rect = Rect::from_xywh(x, y, w, h);
    let (path, anti_alias) = match config.radius {
        Some(r) => (
            PathBuilder::new()
                .add_rrect(RRect::new_rect_xy(rect, r, r), PathDirection::CW, None)
                .detach(),
            true,
        ),
        None => (
            PathBuilder::new()
                .add_rect(rect, PathDirection::CW, None)
                .detach(),
            false,
        ),
    };

    // The shadow.
    // TODO: These shadow definitions need to be fixed.
    match &config.shadow {
        Some(Shadow::XS) => {
            canvas.draw_shadow(
                &path,
                (0., 0., 4.0),
                (0., -600., 600.),
                1600.0,
                colors::scaled_alpha(Color::from_argb(40, 0, 0, 0), opacity),
                colors::scaled_alpha(Color::from_argb(20, 0, 0, 0), opacity),
                ShadowFlags::empty(),
            );
        }
        Some(Shadow::SM) => {
            canvas.draw_shadow(
                &path,
                (0., 0., 8.0),
                (0., -900., 900.),
                2200.0,
                colors::scaled_alpha(Color::from_argb(50, 0, 0, 0), opacity),
                colors::scaled_alpha(Color::from_argb(30, 0, 0, 0), opacity),
                ShadowFlags::empty(),
            );
        }
        None => {}
    };

    // The base.
    let mut fill_paint = Paint::default();
    canvas.draw_path(
        &path,
        &fill_paint
            .set_style(PaintStyle::Fill)
            .set_color(colors::scaled_alpha(config.bg, opacity))
            .set_anti_alias(anti_alias),
    );

    // The border.
    if let Some(b) = &config.border {
        let mut border_paint = Paint::default();
        canvas.draw_path(
            &path,
            &border_paint
                .set_style(PaintStyle::Stroke)
                .set_anti_alias(true)
                .set_stroke(true)
                .set_stroke_width(b.width)
                .set_color(colors::scaled_alpha(b.color, opacity))
                .set_anti_alias(anti_alias),
        );
    }
}
