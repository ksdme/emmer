use std::{
    ops::Div,
    time::{Duration, Instant},
};

/// Represents the visual style of an item.
#[derive(Clone, Default, Debug)]
pub struct Style {
    pub w: f32,
    pub h: f32,

    pub x: f32,
    pub y: f32,

    pub box_opacity: f32,
    pub text_opacity: f32,
}

impl Style {
    pub fn visible(&self) -> bool {
        self.box_opacity > 0.
    }
}

impl Into<PartialStyle> for Style {
    fn into(self) -> PartialStyle {
        PartialStyle {
            w: Some(self.w),
            h: Some(self.h),

            x: Some(self.x),
            y: Some(self.y),

            box_opacity: Some(self.box_opacity),
            text_opacity: Some(self.text_opacity),
        }
    }
}

/// Represents a style object that can be used for property
/// based transitions.
#[derive(Debug, Default)]
pub struct PartialStyle {
    pub w: Option<f32>,
    pub h: Option<f32>,

    pub x: Option<f32>,
    pub y: Option<f32>,

    pub box_opacity: Option<f32>,
    pub text_opacity: Option<f32>,
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
            start_at: start_at.unwrap_or_else(|| Instant::now()),
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
            .as_secs_f32()
            .div(self.duration.as_secs_f32())
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
