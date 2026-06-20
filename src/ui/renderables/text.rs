use crate::ui::renderables::Color;

/// Renders a piece of text using pango with caching.
#[derive(Debug)]
pub struct TextRenderable {
    layout: pango::Layout,
    w: f64,
    h: f64,
}

impl TextRenderable {
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
            layout.set_height(h * pango::SCALE);
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

    pub fn content_size(&self) -> (f64, f64) {
        (self.w, self.h)
    }

    pub fn render(&self, cr: &cairo::Context, x: f64, y: f64, fg: Color) -> (f64, f64) {
        cr.set_source_rgba(fg.r, fg.g, fg.b, fg.a);
        cr.move_to(x, y);

        pangocairo::functions::show_layout(cr, &self.layout);
        self.content_size()
    }
}
