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
    duration: Duration,
    started_at: Instant,
    target: PartialStyle,
}

macro_rules! interp {
    ($target:expr, $current:expr, $progress:expr) => {
        if let Some(target) = $target
            && target != $current
        {
            $current + (target - $current) * $progress
        } else {
            $current
        }
    };
}

impl Transition {
    /// Returns a new transition to target_state with the clock starting now.
    pub fn new(duration: Duration, target: PartialStyle) -> Self {
        Self {
            started_at: Instant::now(),
            duration,
            target,
        }
    }

    /// Interpolates the transition to an Instant.
    pub fn interpolate(&self, current: &Style, now: Option<Instant>) -> (Style, bool) {
        // The progress of the transition as [0, 1].
        let progress = now
            .unwrap_or(Instant::now())
            .duration_since(self.started_at)
            .as_secs_f32()
            .div(self.duration.as_secs_f32())
            .clamp(0., 1.);

        // Return an interpolated visual state along with a boolean indicating if the
        // transition is complete.
        let target = &self.target;
        (
            Style {
                w: interp!(target.w, current.w, progress),
                h: interp!(target.h, current.h, progress),
                x: interp!(target.x, current.x, progress),
                y: interp!(target.y, current.y, progress),
                box_opacity: interp!(target.box_opacity, current.box_opacity, progress),
                text_opacity: interp!(target.text_opacity, current.text_opacity, progress),
            },
            progress >= 1.,
        )
    }
}
