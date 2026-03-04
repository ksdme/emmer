use std::{collections::VecDeque, f64::consts::PI};

use wide::u32x4;

/// Represents a color in cairo as ARGB (0..1).
/// TODO: These should be single byte values.
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

/// Represents a border configuration.
pub struct Border {
    pub color: Color,
    pub width: f64,
}

/// Represents a drop shadow configuration.
pub struct Shadow {
    pub offset: (f64, f64),
    pub blur: u8,
    pub spread: f64,
    pub color: Color,
}

/// Represents a blank rounded card.
pub struct Card {
    pub bg: Color,
    pub border: Option<Border>,
    pub radius: Option<f64>,
    pub shadow: Option<Shadow>,
}

impl Card {
    pub fn draw(&self, cx: cairo::Context, x: f64, y: f64, w: f64, h: f64) {
        if let Some(shadow) = &self.shadow {
            let shadow_surface = {
                let mut off_surface = cairo::ImageSurface::create(
                    cairo::Format::ARgb32,
                    (w + 2.0 * shadow.spread) as i32,
                    (h + 2.0 * shadow.spread) as i32,
                )
                .expect("cairo offscreen");

                {
                    let off_cx = cairo::Context::new(&off_surface).expect("cairo context");

                    // Add path.
                    if let Some(radius) = self.radius {
                        rounded_rect(&off_cx, shadow.spread, shadow.spread, w, h, radius);
                    } else {
                        off_cx.rectangle(shadow.spread, shadow.spread, w, h);
                    }

                    // Fill the color.
                    let color = &shadow.color;
                    off_cx.set_source_rgba(color.r, color.g, color.b, color.a);
                    off_cx.fill().expect("cairo offscreen fill");
                }

                // Blur the surface.
                {
                    off_surface.flush();

                    let w = off_surface.width() as usize;
                    let h = off_surface.height() as usize;

                    let mut pixels = off_surface.data().expect("cairo offscreen pixels");
                    blur(&mut pixels, w, h, shadow.blur.into());
                }

                off_surface
            };

            // Apply the shadow to the main canvas.
            cx.set_source_surface(
                &shadow_surface,
                x + shadow.offset.0 - shadow.spread,
                y + shadow.offset.1 - shadow.spread,
            )
            .expect("cairo offscreen surface");

            cx.paint().expect("cairo pain offscreen");
        }

        // Add base rectangle path.
        if let Some(radius) = self.radius {
            rounded_rect(&cx, x, y, w, h, radius.into());
        } else {
            cx.rectangle(x, y, w, h);
        }

        // Add background.
        let bg = &self.bg;
        cx.set_source_rgba(bg.r, bg.g, bg.b, bg.a);
        cx.fill_preserve().expect("cairo fill");

        // Add the border.
        if let Some(border) = &self.border {
            cx.set_line_width(border.width.into());

            let color = &border.color;
            cx.set_source_rgba(color.r, color.g, color.b, color.a);

            cx.stroke_preserve().expect("cairo stroke");
        }
    }
}

/// Pushes a rounded rectangle to the context.
pub fn rounded_rect(cx: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    cx.new_sub_path();
    cx.arc(x + r, y + r, r, PI, 3.0 * PI / 2.0);
    cx.arc(x + w - r, y + r, r, 3.0 * PI / 2.0, 2.0 * PI);
    cx.arc(x + w - r, y + h - r, r, 0.0, PI / 2.0);
    cx.arc(x + r, y + h - r, r, PI / 2.0, PI);
    cx.close_path();
}

#[inline]
fn index(w: usize, x: usize, y: usize) -> usize {
    (4 * w * y) + (x * 4)
}

#[inline]
fn pixel(data: &mut [u8], w: usize, x: usize, y: usize) -> &mut [u8] {
    let i = index(w, x, y);
    &mut data[i..i + 4]
}

#[inline]
fn p_pixel(data: &mut [u8], w: usize, x: usize, y: usize) -> u32x4 {
    let i = index(w, x, y);
    let px = &data[i..i + 4];
    u32x4::new([px[0] as u32, px[1] as u32, px[2] as u32, px[3] as u32])
}

fn blur(data: &mut [u8], w: usize, h: usize, blur: usize) {
    let mut sum: u32x4;
    let mut stack = VecDeque::<u32x4>::new();

    for y in 0..h {
        sum = u32x4::ZERO;

        stack.clear();
        for x in 0..blur {
            let p_px = p_pixel(data, w, x, y);
            stack.push_back(p_px);
            sum += p_px;
        }

        for x in 0..w {
            if x + blur < w {
                let p_px = p_pixel(data, w, x + blur, y);
                stack.push_back(p_px);
                sum += p_px;
            }

            if x > blur {
                if let Some(p_px) = stack.pop_front() {
                    sum -= p_px;
                }
            }

            let px = pixel(data, w, x, y);
            let lanes = sum.as_array();
            let count = stack.len() as u32;
            px[0] = (lanes[0] / count) as u8;
            px[1] = (lanes[1] / count) as u8;
            px[2] = (lanes[2] / count) as u8;
            px[3] = (lanes[3] / count) as u8;
        }
    }

    for x in 0..w {
        sum = u32x4::ZERO;

        stack.clear();
        for y in 0..blur {
            let p_px = p_pixel(data, w, x, y);
            stack.push_back(p_px);
            sum += p_px;
        }

        for y in 0..h {
            if y + blur < h {
                let p_px = p_pixel(data, w, x, y + blur);
                stack.push_back(p_px);
                sum += p_px;
            }

            if y > blur {
                if let Some(p_px) = stack.pop_front() {
                    sum -= p_px;
                }
            }

            let px = pixel(data, w, x, y);
            let lanes = sum.as_array();
            let count = stack.len() as u32;
            px[0] = (lanes[0] / count) as u8;
            px[1] = (lanes[1] / count) as u8;
            px[2] = (lanes[2] / count) as u8;
            px[3] = (lanes[3] / count) as u8;
        }
    }
}
