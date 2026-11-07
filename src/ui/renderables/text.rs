use anyhow::Result;

use crate::ui::renderables::{Color, Rect, Renderable, Size};

/// Renders a piece of text using pango with caching.
#[derive(Debug)]
pub struct Text {
    layout: pango::Layout,
    w: f64,
    h: f64,
}

impl Text {
    /// Returns a new Text renderable that caches the content shape and layout
    /// while constraing and wrapping it.
    pub fn new(
        font_map: &pango::FontMap,
        font_description: &pango::FontDescription,
        max_w: Option<i32>,
        max_h: Option<i32>,
        text: &str,
    ) -> Self {
        let cx = pango::Context::new();
        cx.set_font_map(Some(font_map));

        let layout = pango::Layout::new(&cx);

        if let Some(w) = max_w {
            layout.set_width(w * pango::SCALE);
        }
        if let Some(h) = max_h {
            if h < 0 {
                // Limit to the number of lines of text instead.
                layout.set_height(h);
                layout.set_ellipsize(pango::EllipsizeMode::End);
            } else {
                layout.set_height(h * pango::SCALE);
            }
        }
        layout.set_wrap(pango::WrapMode::Word);

        layout.set_font_description(Some(font_description));
        layout.set_text(text);

        let (w, h) = layout.pixel_size();
        Self {
            layout,
            w: w as f64,
            h: h as f64,
        }
    }
}

impl Renderable for Text {
    type Style = Style;

    fn measure(&self) -> Size {
        Size {
            w: self.w,
            h: self.h,
        }
    }

    fn paint(&self, cr: &cairo::Context, rect: Rect, style: &Style) -> Result<()> {
        let fg = style.color;
        cr.set_source_rgba(fg.r, fg.g, fg.b, fg.a);
        cr.move_to(rect.x1, rect.y1);

        pangocairo::functions::show_layout(cr, &self.layout);
        Ok(())
    }
}

/// The style of a text renderable.
#[derive(Debug, Default, Clone, Copy)]
pub struct Style {
    pub color: Color,
}
