use std::{
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

    pub fn lighter(&self, amount: f64) -> Color {
        Color {
            r: self.r + (1. - self.r) * amount,
            g: self.g + (1. - self.g) * amount,
            b: self.b + (1. - self.b) * amount,
            a: self.a + (1. - self.a) * amount,
        }
    }
}

/// Represents a rect used to represent hitboxes/bounds.
#[derive(Debug, Default, Clone, Copy)]
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

    pub fn w(&self) -> f64 {
        self.x2 - self.x1
    }

    pub fn h(&self) -> f64 {
        self.y2 - self.y1
    }

    /// Returns a boolean indicating if the (x, y) is on or within the rect.
    #[inline]
    pub fn contains(&self, at: (f64, f64)) -> bool {
        at.0 >= self.x1 && at.0 <= self.x2 && at.1 >= self.y1 && at.1 <= self.y2
    }
}

// A trait that allows us to provide implementations for how to interpolate values
// of each type from a value a to optional b.
pub trait Interp: Copy {
    fn interp(self, to: Option<Self>, frac: f64) -> Self;
}

impl Interp for f64 {
    #[inline]
    fn interp(self, to: Option<f64>, frac: f64) -> f64 {
        if let Some(to) = to
            && self != to
        {
            self + (to - self) * frac
        } else {
            self
        }
    }
}

impl Interp for Color {
    #[inline]
    fn interp(self, to: Option<Color>, frac: f64) -> Color {
        if let Some(to) = to
            && self != to
        {
            Color {
                r: (self.r + (to.r - self.r) * frac).clamp(0., 1.),
                g: (self.g + (to.g - self.g) * frac).clamp(0., 1.),
                b: (self.b + (to.b - self.b) * frac).clamp(0., 1.),
                a: (self.a + (to.a - self.a) * frac).clamp(0., 1.),
            }
        } else {
            self
        }
    }
}

/// Reprenents an implementation of Style that can be interpolated.
/// This is a requirement for a style that needs to be transitionable.
pub trait Transitionable: Clone {
    type Partial;

    fn interpolate(&self, to: &Self::Partial, progress: f64) -> Self;
}

/// A macro to define a style with a partial variant and a transition model.
#[macro_export]
macro_rules! transitionable {
    (
        $name:ident {
            $($field:ident : $ty:ty),* $(,)?
        }
    ) => {
        #[derive(Clone, Debug, Default)]
        pub struct $name {
            $(pub $field: $ty,)*
        }

        pastey::paste! {
            #[derive(Debug, Default)]
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

            impl $crate::ui::renderables::Transitionable for $name {
                type Partial = [<Partial $name>];

                fn interpolate(&self, to: &Self::Partial, progress: f64) -> $name {
                    $name {
                        $(
                            $field: crate::ui::renderables::Interp::interp(self.$field, to.$field, progress),
                        )*
                    }
                }
            }

            pub type [<$name Transition>] = crate::ui::renderables::Transition<$name>;
        }
    };
}

/// Represents the parameters of a transition of a style into another.
#[derive(Debug)]
pub struct Transition<S: Transitionable> {
    starts: Instant,
    duration: Duration,

    from: Option<S>,
    to: S::Partial,
}

impl<S> Transition<S>
where
    S: Transitionable,
{
    /// Returns a new transition to target_state with a shared clock.
    pub fn new(duration: Duration, to: S::Partial, starts: Option<Instant>) -> Self {
        Self {
            starts: starts.unwrap_or_else(Instant::now),
            duration,

            from: None,
            to,
        }
    }

    /// Interpolates the transition to an Instant.
    pub fn interpolate(&mut self, current: &S, now: &Instant) -> (S, bool) {
        // The progress of the transition as [0, 1].
        let progress = now
            .checked_duration_since(self.starts)
            .unwrap_or_default()
            .as_secs_f64()
            .div(self.duration.as_secs_f64())
            .clamp(0., 1.);

        // The first interpolation request is treated as the starting point of
        // the transition.
        let from = self.from.get_or_insert_with(|| current.clone());

        // Return an interpolated visual state along with a boolean indicating if the
        // transition is complete.
        (from.interpolate(&self.to, progress), progress >= 1.)
    }
}

pub mod button;
pub mod card;
pub mod notification;
pub mod text;
