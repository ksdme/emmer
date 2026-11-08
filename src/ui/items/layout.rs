use crate::{
    config::{SpreadConfig, StackConfig},
    transitionable,
    ui::{
        anchors::{Anchor, VerticalAnchor},
        items::stack::Presentation,
        renderables::{Rect, Size, notification},
    },
};

/// What the layout needs to know about an item.
#[derive(Debug, Clone, Copy)]
pub struct Input {
    /// The intrinsic size of the notification card.
    pub size: Size,

    /// The height of the action buttons block below the card, or 0 if the
    /// buttons should not be shown.
    pub buttons_h: f64,

    pub dismissed: bool,
}

transitionable!(
    /// Where and how an item should end up.
    Target {
        rect: Rect,
        style: notification::Style,
        buttons: ButtonsTarget,
    }
);

transitionable!(
    /// How the action buttons below a card should end up.
    ButtonsTarget {
        opacity: f64,
    }
);

/// The parameters of a layout pass.
///
/// All the positions are computed as a distance from the anchored edge of the
/// layer and are only flipped into layer coordinates at the very end.
pub struct Layout<'a> {
    pub anchor: &'a Anchor,
    pub presentation: Presentation,

    pub margin_x: f64,
    pub stack: &'a StackConfig,
    pub spread: &'a SpreadConfig,

    /// The distance from the anchored edge at which the stack starts.
    pub origin: f64,
}

impl Layout<'_> {
    /// Returns the targets of the items, which are expected to be ordered from
    /// the newest to the oldest.
    pub fn targets(&self, items: impl IntoIterator<Item = Input>) -> Vec<Target> {
        let max_count = match self.presentation {
            Presentation::Stacked => self.stack.max_count,
            Presentation::Spread => self.spread.max_count,
        };

        // The distance at which the next item can start without overlapping the
        // previous one, and the number of visible items laid out so far.
        let mut d = self.origin;
        let mut no = 0;

        items
            .into_iter()
            .map(|item| {
                let shown = no < max_count && !item.dismissed;
                let (target, next) = match self.presentation {
                    Presentation::Stacked => self.stacked(item, no, shown, d),
                    Presentation::Spread => self.spread(item, shown, d),
                };

                // Hidden and dismissed items give up their position to the next item.
                if shown {
                    no += 1;
                    d = next;
                }

                target
            })
            .collect()
    }

    /// Returns the rect a new item should appear from, just outside the stack.
    pub fn spawn(&self, size: Size) -> Rect {
        let d = self.origin - self.spread.gap - size.h;
        Rect::from_xywh(self.margin_x, self.flip(d, size.h), size.w, size.h)
    }

    // Returns the target of a stacked item at position no and the distance the
    // next item starts at.
    fn stacked(&self, item: Input, no: usize, shown: bool, d: f64) -> (Target, f64) {
        let peek = self.stack.peek;
        let inset = no as f64 * self.stack.inset;
        let Size { w, h } = item.size;

        let (near, h, next) = if no == 0 {
            // The front card takes its full size.
            (d, h, d + h)
        } else {
            // The other cards are tucked behind the previous one with only their
            // far edge peeking out. Since a card could be taller than the ones
            // before it, it is clipped so that it never sticks out past the
            // front card.
            let h = h.min(d - self.origin - peek).max(0.);
            (d + peek - h, h, d + peek)
        };
        let target = Target {
            rect: Rect::from_xywh(self.margin_x + inset, self.flip(near, h), w - 2. * inset, h),
            style: notification::Style {
                outer_opacity: opacity(shown),
                inner_opacity: opacity(shown && no == 0),
            },
            buttons: ButtonsTarget { opacity: 0. },
        };

        (target, next)
    }

    // Returns the target of a spread item and the distance the next item starts
    // at.
    fn spread(&self, item: Input, shown: bool, d: f64) -> (Target, f64) {
        let gap = self.spread.gap;
        let Size { w, h } = item.size;

        // The card and the buttons below it are laid out as one block.
        let buttons_h = if shown { item.buttons_h } else { 0. };
        let block_h = h + buttons_h;

        // A dismissed card slides out towards the anchored edge.
        let near = if item.dismissed { d - h - gap } else { d };
        let target = Target {
            rect: Rect::from_xywh(self.margin_x, self.flip(near, block_h), w, h),
            style: notification::Style {
                outer_opacity: opacity(shown),
                inner_opacity: opacity(shown),
            },
            buttons: ButtonsTarget {
                opacity: opacity(buttons_h > 0.),
            },
        };

        (target, d + block_h + gap)
    }

    // Converts a distance from the anchored edge to the y of a box of height h.
    fn flip(&self, d: f64, h: f64) -> f64 {
        match self.anchor.vertical {
            VerticalAnchor::Top => d,
            VerticalAnchor::Bottom => self.anchor.layer_height - d - h,
        }
    }
}

fn opacity(visible: bool) -> f64 {
    if visible { 1. } else { 0. }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::anchors::HorizontalAnchor;

    const STACK: StackConfig = StackConfig {
        peek: 8.,
        inset: 8.,
        max_count: 3,
    };
    const SPREAD: SpreadConfig = SpreadConfig {
        gap: 8.,
        max_count: 20,
    };

    fn anchor(vertical: VerticalAnchor) -> Anchor {
        Anchor {
            horizontal: HorizontalAnchor::Right,
            vertical,
            layer_width: 400.,
            layer_height: 1000.,
        }
    }

    fn layout(anchor: &Anchor, presentation: Presentation) -> Layout<'_> {
        Layout {
            anchor,
            presentation,
            margin_x: 32.,
            stack: &STACK,
            spread: &SPREAD,
            origin: 32.,
        }
    }

    fn input(h: f64, dismissed: bool) -> Input {
        Input {
            size: Size { w: 320., h },
            buttons_h: 0.,
            dismissed,
        }
    }

    // Mirrors a top anchored rect into a bottom anchored one.
    fn mirror(rect: Rect, layer_h: f64) -> Rect {
        Rect::from_xywh(rect.x1, layer_h - rect.y2, rect.w(), rect.h())
    }

    #[test]
    fn anchors_mirror_each_other() {
        let items = [
            input(80., false),
            input(120., true),
            input(60., false),
            input(100., false),
            input(90., false),
        ];

        for presentation in [Presentation::Stacked, Presentation::Spread] {
            let top = anchor(VerticalAnchor::Top);
            let bottom = anchor(VerticalAnchor::Bottom);

            let top = layout(&top, presentation).targets(items);
            let bottom = layout(&bottom, presentation).targets(items);

            for (t, b) in top.iter().zip(bottom.iter()) {
                assert_eq!(mirror(t.rect, 1000.), b.rect, "{presentation:?}");
                assert_eq!(t.style, b.style);
            }
        }
    }

    #[test]
    fn stacked_cards_never_stick_out_past_the_origin() {
        let anchor = anchor(VerticalAnchor::Top);
        let targets = layout(&anchor, Presentation::Stacked).targets([
            input(40., false),
            input(200., false),
            input(200., false),
        ]);

        for target in &targets {
            assert!(target.rect.y1 >= 32., "{target:?}");
        }
        assert_eq!(targets[1].rect.y2, 32. + 40. + 8.);
    }

    #[test]
    fn dismissed_spread_card_slides_out_by_its_height() {
        let top = anchor(VerticalAnchor::Top);
        let bottom = anchor(VerticalAnchor::Bottom);

        let top = layout(&top, Presentation::Spread).targets([input(80., true)]);
        let bottom = layout(&bottom, Presentation::Spread).targets([input(80., true)]);

        assert_eq!(top[0].rect.y1, 32. - 80. - 8.);
        assert_eq!(bottom[0].rect.y1, 1000. - 32. + 8.);
    }

    #[test]
    fn spawn_uses_the_origin() {
        let anchor = anchor(VerticalAnchor::Bottom);
        let mut layout = layout(&anchor, Presentation::Stacked);
        layout.origin = 64.;

        let rect = layout.spawn(Size { w: 320., h: 80. });
        assert_eq!(rect.y1, 1000. - 64. + 8.);
    }
}
