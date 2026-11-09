use std::{
    f64::consts::PI,
    ops::Div,
    time::{Duration, Instant},
};

/// Represents a color.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub fn from_rgba_u8(r: u8, g: u8, b: u8, a: f64) -> Self {
        Self {
            r: r as f64 / 255.,
            g: g as f64 / 255.,
            b: b as f64 / 255.,
            a,
        }
    }

    pub fn with_lightness(&self, amount: f64) -> Color {
        Color {
            r: self.r + (1. - self.r) * amount,
            g: self.g + (1. - self.g) * amount,
            b: self.b + (1. - self.b) * amount,
            a: self.a,
        }
    }

    pub fn with_alpha(&self, a: f64) -> Color {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a,
        }
    }
}

/// Represents the intrinsic size of a renderable.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Size {
    pub w: f64,
    pub h: f64,
}

/// Represents a rect used to represent hitboxes/bounds.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x1: f64,
    pub y1: f64,

    pub x2: f64,
    pub y2: f64,
}

impl Rect {
    pub fn from_xywh(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self {
            x1: x,
            y1: y,
            x2: (x + w),
            y2: (y + h),
        }
    }

    pub fn from_origin(x: f64, y: f64, size: Size) -> Self {
        Self::from_xywh(x, y, size.w, size.h)
    }

    pub fn w(&self) -> f64 {
        self.x2 - self.x1
    }

    pub fn h(&self) -> f64 {
        self.y2 - self.y1
    }

    /// Returns the smallest rect that contains both the rects.
    pub fn union(&self, other: &Rect) -> Rect {
        Rect {
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
            x2: self.x2.max(other.x2),
            y2: self.y2.max(other.y2),
        }
    }

    /// Returns the rect moved by (dx, dy).
    pub fn translate(&self, dx: f64, dy: f64) -> Rect {
        Rect {
            x1: self.x1 + dx,
            y1: self.y1 + dy,
            x2: self.x2 + dx,
            y2: self.y2 + dy,
        }
    }

    /// Returns the overlap between both the rects, if there is any.
    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        let rect = Rect {
            x1: self.x1.max(other.x1),
            y1: self.y1.max(other.y1),
            x2: self.x2.min(other.x2),
            y2: self.y2.min(other.y2),
        };
        (rect.x1 < rect.x2 && rect.y1 < rect.y2).then_some(rect)
    }

    /// Returns a boolean indicating if the (x, y) is on or within the rect.
    #[inline]
    pub fn contains(&self, at: (f64, f64)) -> bool {
        at.0 >= self.x1 && at.0 <= self.x2 && at.1 >= self.y1 && at.1 <= self.y2
    }
}

impl Interp for Rect {
    #[inline]
    fn interp(self, to: Rect, frac: f64) -> Rect {
        Rect {
            x1: self.x1.interp(to.x1, frac),
            y1: self.y1.interp(to.y1, frac),
            x2: self.x2.interp(to.x2, frac),
            y2: self.y2.interp(to.y2, frac),
        }
    }
}

// A trait that allows us to provide implementations for how to interpolate values
// of each type from a value a to b.
pub trait Interp: Copy + PartialEq {
    fn interp(self, to: Self, frac: f64) -> Self;
}

impl Interp for f64 {
    #[inline]
    fn interp(self, to: f64, frac: f64) -> f64 {
        self + (to - self) * frac
    }
}

impl Interp for Color {
    #[inline]
    fn interp(self, to: Color, frac: f64) -> Color {
        Color {
            r: self.r.interp(to.r, frac).clamp(0., 1.),
            g: self.g.interp(to.g, frac).clamp(0., 1.),
            b: self.b.interp(to.b, frac).clamp(0., 1.),
            a: self.a.interp(to.a, frac).clamp(0., 1.),
        }
    }
}

/// Maps the linear progress of a transition onto a curve.
#[allow(dead_code)]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Easing {
    #[default]
    Linear,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
}

impl Easing {
    /// Returns the eased progress for a linear progress in [0, 1].
    pub fn apply(self, t: f64) -> f64 {
        match self {
            Easing::Linear => t,
            Easing::EaseInCubic => t * t * t,
            Easing::EaseOutCubic => 1. - (1. - t).powi(3),
            Easing::EaseInOutCubic => {
                if t < 0.5 {
                    4. * t * t * t
                } else {
                    1. - (2. - 2. * t).powi(3) / 2.
                }
            }
        }
    }
}

/// The duration and the curve of a transition.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Timing {
    pub duration: Duration,
    pub easing: Easing,
}

impl Timing {
    pub const fn new(duration: Duration, easing: Easing) -> Self {
        Self { duration, easing }
    }
}

/// A value with its own transition timeline.
#[derive(Debug, Clone)]
pub struct Animated<T: Interp> {
    // The value as of the last tick.
    value: T,

    from: T,
    to: T,
    starts: Instant,
    timing: Timing,
}

impl<T: Interp> Animated<T> {
    /// Returns a value that is settled at value.
    pub fn new(value: T) -> Self {
        Self {
            value,

            from: value,
            to: value,
            starts: Instant::now(),
            timing: Timing::default(),
        }
    }

    /// The value as of the last tick.
    pub fn get(&self) -> T {
        self.value
    }

    /// The value the transition is heading towards.
    pub fn target(&self) -> T {
        self.to
    }

    /// Starts a transition from wherever the value is at now towards to. If the
    /// value is already heading towards to, the transition underway is kept.
    pub fn animate_to(&mut self, to: T, timing: Timing, now: Instant) {
        if to == self.to {
            return;
        }

        self.from = self.at(now);
        self.to = to;
        self.starts = now;
        self.timing = timing;
    }

    /// Progresses the value to now and returns a boolean indicating if the
    /// transition has completed.
    pub fn tick(&mut self, now: Instant) -> bool {
        self.value = self.at(now);
        self.value == self.to
    }

    // The value of the transition at an Instant.
    fn at(&self, now: Instant) -> T {
        let duration = self.timing.duration.as_secs_f64();
        let progress = if duration > 0. {
            now.checked_duration_since(self.starts)
                .unwrap_or_default()
                .as_secs_f64()
                .div(duration)
                .clamp(0., 1.)
        } else {
            1.
        };

        if progress >= 1. {
            self.to
        } else {
            self.from
                .interp(self.to, self.timing.easing.apply(progress))
        }
    }
}

impl<T: Interp + Default> Default for Animated<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

/// A macro to define a style along with a partial variant and an animated variant
/// that has an independent timeline for each of its fields.
#[macro_export]
macro_rules! transitionable {
    (
        $(#[$meta:meta])*
        $name:ident {
            $($field:ident : $ty:ty),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Default, PartialEq)]
        pub struct $name {
            $(pub $field: $ty,)*
        }

        pastey::paste! {
            #[allow(dead_code)]
            #[derive(Debug, Default, Clone)]
            pub struct [<Partial $name>] {
                $(pub $field: Option<$ty>,)*
            }

            impl From<$name> for [<Partial $name>] {
                fn from(val: $name) -> Self {
                    Self {
                        $(
                            $field: Some(val.$field),
                        )*
                    }
                }
            }

            // Allows the style to be nested as a single field of another style.
            impl $crate::ui::renderables::Interp for $name {
                #[inline]
                fn interp(self, to: Self, frac: f64) -> Self {
                    $name {
                        $(
                            $field: $crate::ui::renderables::Interp::interp(self.$field, to.$field, frac),
                        )*
                    }
                }
            }

            #[allow(dead_code)]
            #[derive(Debug, Default, Clone)]
            pub struct [<Animated $name>] {
                $(pub $field: $crate::ui::renderables::Animated<$ty>,)*
            }

            #[allow(dead_code)]
            impl [<Animated $name>] {
                pub fn new(value: $name) -> Self {
                    Self {
                        $(
                            $field: $crate::ui::renderables::Animated::new(value.$field),
                        )*
                    }
                }

                /// The value as of the last tick.
                pub fn get(&self) -> $name {
                    $name {
                        $(
                            $field: self.$field.get(),
                        )*
                    }
                }

                /// The value the transitions are heading towards.
                pub fn target(&self) -> $name {
                    $name {
                        $(
                            $field: self.$field.target(),
                        )*
                    }
                }

                /// Retargets only the fields that are set on to.
                pub fn animate_to(
                    &mut self,
                    to: impl Into<[<Partial $name>]>,
                    timing: $crate::ui::renderables::Timing,
                    now: std::time::Instant,
                ) {
                    let to = to.into();
                    $(
                        if let Some(value) = to.$field {
                            self.$field.animate_to(value, timing, now);
                        }
                    )*
                }

                /// Progresses all the fields to now and returns a boolean indicating
                /// if all of their transitions have completed.
                pub fn tick(&mut self, now: std::time::Instant) -> bool {
                    let mut complete = true;
                    $(
                        complete &= self.$field.tick(now);
                    )*
                    complete
                }
            }
        }
    };
}

/// The contract shared by all the renderables. The layout measures a renderable,
/// decides the rect it occupies and then asks it to paint itself into that rect.
pub trait Renderable {
    /// The paint properties of the renderable.
    type Style;

    /// Returns the intrinsic size of the renderable.
    fn measure(&self) -> Size;

    /// Paints the renderable into the rect.
    fn paint(&self, cr: &cairo::Context, rect: Rect, style: &Self::Style) -> anyhow::Result<()>;
}

/// A util for setting up a cairo rounded rect path. You could use cr.arc but
/// that depends on starting position which can get tricky to get right.
/// Maybe we can put this in a better place.
pub fn rounded_sub_path(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    cr.new_sub_path();
    cr.arc(x + r, y + r, r, PI, 3.0 * PI / 2.0);
    cr.arc(x + w - r, y + r, r, 3.0 * PI / 2.0, 2.0 * PI);
    cr.arc(x + w - r, y + h - r, r, 0.0, PI / 2.0);
    cr.arc(x + r, y + h - r, r, PI / 2.0, PI);
    cr.close_path();
}

pub mod bar;
pub mod button;
pub mod card;
pub mod image;
pub mod notification;
pub mod text;

#[cfg(test)]
mod tests {
    use super::*;

    const LINEAR: Timing = Timing::new(Duration::from_millis(100), Easing::Linear);

    transitionable!(Pair { a: f64, b: f64 });

    fn ms(at: Instant, millis: u64) -> Instant {
        at + Duration::from_millis(millis)
    }

    #[test]
    fn from_is_captured_when_the_transition_starts() {
        let t0 = Instant::now();
        let mut value = Animated::new(0.);
        value.animate_to(1., LINEAR, t0);

        // A late first frame lands where the clock says, not at the start.
        assert!(!value.tick(ms(t0, 50)));
        assert_eq!(value.get(), 0.5);
    }

    #[test]
    fn retargeting_continues_from_the_current_value() {
        let t0 = Instant::now();
        let mut value = Animated::new(0.);
        value.animate_to(1., LINEAR, t0);

        // Retarget without ticking in between.
        value.animate_to(0., LINEAR, ms(t0, 50));
        value.tick(ms(t0, 50));
        assert_eq!(value.get(), 0.5);

        value.tick(ms(t0, 100));
        assert_eq!(value.get(), 0.25);
    }

    #[test]
    fn retargeting_to_the_same_target_keeps_the_transition() {
        let t0 = Instant::now();
        let mut value = Animated::new(0.);
        value.animate_to(1., LINEAR, t0);
        value.animate_to(1., LINEAR, ms(t0, 50));

        assert!(value.tick(ms(t0, 100)));
        assert_eq!(value.get(), 1.);
    }

    #[test]
    fn partial_updates_only_retarget_the_fields_they_touch() {
        let t0 = Instant::now();
        let mut pair = AnimatedPair::new(Pair { a: 0., b: 0. });
        pair.animate_to(Pair { a: 1., b: 1. }, LINEAR, t0);

        pair.animate_to(
            PartialPair {
                a: None,
                b: Some(0.),
            },
            LINEAR,
            ms(t0, 50),
        );

        pair.tick(ms(t0, 100));
        assert_eq!(pair.get(), Pair { a: 1., b: 0.25 });
    }

    #[test]
    fn styles_nest_as_a_single_field() {
        transitionable!(Outer {
            inner: Pair,
            c: f64
        });

        let t0 = Instant::now();
        let mut outer = AnimatedOuter::new(Outer::default());
        outer.animate_to(
            Outer {
                inner: Pair { a: 1., b: 0.5 },
                c: 1.,
            },
            LINEAR,
            t0,
        );

        outer.tick(ms(t0, 50));
        assert_eq!(outer.get().inner, Pair { a: 0.5, b: 0.25 });
    }

    #[test]
    fn easings_start_and_end_at_the_bounds() {
        for easing in [
            Easing::Linear,
            Easing::EaseInCubic,
            Easing::EaseOutCubic,
            Easing::EaseInOutCubic,
        ] {
            assert_eq!(easing.apply(0.), 0., "{easing:?}");
            assert_eq!(easing.apply(1.), 1., "{easing:?}");
        }
    }
}
