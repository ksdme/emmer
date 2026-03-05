use std::collections::VecDeque;

use wide::u32x4;

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

pub fn stack_blur(data: &mut [u8], w: usize, h: usize, blur: usize) {
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

            if x > blur
                && let Some(p_px) = stack.pop_front() {
                    sum -= p_px;
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

            if y > blur
                && let Some(p_px) = stack.pop_front() {
                    sum -= p_px;
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
