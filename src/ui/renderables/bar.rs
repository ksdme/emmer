use std::rc::Rc;

use anyhow::{Context, Result};

use crate::{
    config::ComputedConfig,
    ui::renderables::{Rect, Renderable, Size, card, rounded_sub_path, text},
};

/// Whether the stack behind the bar is showing its cards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Expanded,
    Stashed,
}

/// A clickable control on the right side of the bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Clear,
    Stash,
    Expand,
}

impl Control {
    fn label(self) -> &'static str {
        match self {
            Control::Clear => "Clear",
            Control::Stash => "Stash",
            Control::Expand => "Show All",
        }
    }
}

fn controls_for(mode: Mode) -> &'static [Control] {
    match mode {
        Mode::Expanded => &[Control::Stash, Control::Clear],
        Mode::Stashed => &[Control::Expand],
    }
}

/// Renders the dismiss-all bar, which shows the count of the notifications and
/// controls to clear or stash them.
#[derive(Debug)]
pub struct Bar {
    config: Rc<ComputedConfig>,

    count_r: text::Text,
    // Active controls only, left to right.
    controls: Vec<(Control, text::Text)>,
}

impl Bar {
    pub fn new(config: Rc<ComputedConfig>, count: usize, mode: Mode) -> Self {
        let font = &config.theme.bar_label.font;
        let text = |label: &str| text::Text::new(&config.font_map, font, None, Some(-1), label);

        Self {
            count_r: text(&format!("{count} Notifications")),
            controls: controls_for(mode)
                .iter()
                .map(|&control| (control, text(control.label())))
                .collect(),

            config,
        }
    }

    // Walks the controls from the right edge inward.
    fn each_control<E>(
        &self,
        rect: Rect,
        mut f: impl FnMut(Control, &text::Text, Rect) -> Result<(), E>,
    ) -> Result<(), E> {
        let padding = self.config.theme.bar.padding.x;
        let mut x2 = rect.x2;
        for (control, label) in self.controls.iter().rev() {
            let w = 2. * padding + label.measure().w;
            let x1 = x2 - w;
            f(*control, label, Rect { x1, x2, ..rect })?;
            x2 = x1;
        }
        Ok(())
    }

    /// Returns the control at (x, y) when the bar is painted into rect, if any.
    pub fn control_at(&self, rect: Rect, at: (f64, f64)) -> Option<Control> {
        let mut hit = None;
        let _ = self.each_control(rect, |control, _, control_rect| {
            if control_rect.contains(at) {
                hit = Some(control);
            }
            Ok::<(), ()>(())
        });
        hit
    }
}

impl Renderable for Bar {
    type Style = Style;

    fn measure(&self) -> Size {
        let count = self.count_r.measure();
        let control_h = self
            .controls
            .iter()
            .map(|(_, label)| label.measure().h)
            .fold(0., f64::max);
        Size {
            w: self.config.width,
            h: 2. * self.config.theme.bar.padding.y + count.h.max(control_h),
        }
    }

    fn paint(&self, cr: &cairo::Context, rect: Rect, style: &Style) -> Result<()> {
        let theme = &self.config.theme;
        let padding = &theme.bar.padding;
        let box_style = card::Style::from_surface(&theme.bar, style.opacity);
        let border = box_style.border_color;
        let fg = text::Style {
            color: theme.bar_label.color.faded(style.opacity),
        };

        card::paint(cr, rect, &box_style).context("Could not draw box")?;

        // The hover tint on the lit control, clipped to the rounded shape of
        // the bar.
        if style.control_light > 0. && let Some(lit) = style.lit_control {
            let mut lit_rect = None;
            self.each_control(rect, |control, _, control_rect| {
                if control == lit {
                    lit_rect = Some(control_rect);
                }
                Ok::<(), anyhow::Error>(())
            })?;

            if let Some(lit_rect) = lit_rect {
                // TODO: This is not the ideal way of doing this.
                cr.save().context("Could not save the cairo state")?;
                cr.new_path();
                rounded_sub_path(
                    cr,
                    rect.x1,
                    rect.y1,
                    rect.w(),
                    rect.h(),
                    box_style.border_radius,
                );
                cr.clip();

                cr.rectangle(lit_rect.x1, lit_rect.y1, lit_rect.w(), lit_rect.h());
                cr.set_source_rgba(1., 1., 1., style.control_light * style.opacity);
                cr.fill().context("Could not draw control tint")?;

                cr.restore().context("Could not restore the cairo state")?;
            }
        }

        let count = self.count_r.measure();
        self.count_r
            .paint(
                cr,
                Rect::from_origin(
                    rect.x1 + padding.x,
                    rect.y1 + (rect.h() - count.h) / 2.,
                    count,
                ),
                &fg,
            )
            .context("Could not draw count")?;

        self.each_control(rect, |_, label, control_rect| {
            cr.new_path();
            cr.move_to(control_rect.x1, rect.y1);
            cr.line_to(control_rect.x1, rect.y2);
            cr.set_source_rgba(border.r, border.g, border.b, border.a);
            cr.set_line_width(0.5);
            cr.stroke().context("Could not draw divider")?;

            let size = label.measure();
            label
                .paint(
                    cr,
                    Rect::from_origin(
                        control_rect.x1 + padding.x,
                        rect.y1 + (rect.h() - size.h) / 2.,
                        size,
                    ),
                    &fg,
                )
                .context("Could not draw control")
        })
    }
}

/// The style of the bar renderable.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Style {
    pub control_light: f64,
    pub lit_control: Option<Control>,
    pub opacity: f64,
}
