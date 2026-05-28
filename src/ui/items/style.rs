use std::{
    ops::Div,
    time::{Duration, Instant},
};

/// Represents the visual style of an item.
#[derive(Clone, Default, Debug)]
pub struct Style {
    pub w: f64,
    pub h: f64,

    pub x: f64,
    pub y: f64,

    pub box_opacity: f64,
    pub text_opacity: f64,
}

impl Style {
    pub fn visible(&self) -> bool {
        self.box_opacity > 0.
    }
}

impl From<Style> for PartialStyle {
    fn from(val: Style) -> Self {
        PartialStyle {
            w: Some(val.w),
            h: Some(val.h),

            x: Some(val.x),
            y: Some(val.y),

            box_opacity: Some(val.box_opacity),
            text_opacity: Some(val.text_opacity),
        }
    }
}

/// Represents a style object that can be used for property
/// based transitions.
#[derive(Debug, Default)]
pub struct PartialStyle {
    pub w: Option<f64>,
    pub h: Option<f64>,

    pub x: Option<f64>,
    pub y: Option<f64>,

    pub box_opacity: Option<f64>,
    pub text_opacity: Option<f64>,
}

/// Represents the parameters of a transition of a Style into another.
#[derive(Debug)]
pub struct Transition {
    start_at: Instant,
    duration: Duration,

    from: Option<Style>,
    to: PartialStyle,
}

macro_rules! interp {
    ($target:expr, $from:expr, $current:expr, $progress:expr) => {
        if let Some(target) = $target
            && target != $from
        {
            $from + (target - $from) * $progress
        } else {
            $current
        }
    };
}

impl Transition {
    /// Returns a new transition to target_state with a shared clock.
    pub fn new(duration: Duration, to: PartialStyle, start_at: Option<Instant>) -> Self {
        Self {
            start_at: start_at.unwrap_or_else(Instant::now),
            duration,

            from: None,
            to,
        }
    }

    /// Interpolates the transition to an Instant.
    pub fn interpolate(&mut self, current: &Style, now: &Instant) -> (Style, bool) {
        // The progress of the transition as [0, 1].
        let progress = now
            .checked_duration_since(self.start_at)
            .unwrap_or_default()
            .as_secs_f64()
            .div(self.duration.as_secs_f64())
            .clamp(0., 1.);

        // The first interpolation request is treated as the starting point of
        // the transition.
        let from = self.from.get_or_insert_with(|| current.clone());

        // Return an interpolated visual state along with a boolean indicating if the
        // transition is complete.
        let to = &self.to;
        (
            Style {
                w: interp!(to.w, from.w, current.w, progress),
                h: interp!(to.h, from.h, current.h, progress),
                x: interp!(to.x, from.x, current.x, progress),
                y: interp!(to.y, from.y, current.y, progress),
                box_opacity: interp!(
                    to.box_opacity,
                    from.box_opacity,
                    current.box_opacity,
                    progress
                ),
                text_opacity: interp!(
                    to.text_opacity,
                    from.text_opacity,
                    current.text_opacity,
                    progress
                ),
            },
            progress >= 1.,
        )
    }
}
