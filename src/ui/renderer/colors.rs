use skia_safe::Color;

#[inline]
pub fn scaled_alpha(color: Color, opacity: f32) -> Color {
    color.with_a((color.a() as f32 * opacity) as u8)
}
