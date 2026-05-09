use std::{
    ops::Div,
    time::{Duration, Instant},
};

/// Represents the visual style of an item. Style should be the
/// source of information when an item is being drawn to the screen.
#[derive(Clone, Default, Debug)]
pub struct Style {
    pub w: f32,
    pub h: f32,
    pub x: f32,
    pub y: f32,
    pub opacity: f32,
}

impl Style {
    pub fn visible(&self) -> bool {
        self.opacity <= 0.
    }
}

/// Represents the parameters of a transition of a Style into another.
pub struct Transition {
    start_at: Instant,
    duration: Duration,
    target: Style,
}

macro_rules! interp {
    ($target:expr, $current:expr, $progress:expr) => {
        if $target != $current {
            $current + ($target - $current) * $progress
        } else {
            $current
        }
    };
}

impl Transition {
    /// Returns a new transition to target_state with the clock starting now.
    pub fn new(duration: Duration, target: Style, delay: Option<Duration>) -> Self {
        Self {
            start_at: Instant::now() + delay.unwrap_or_default(),
            duration,
            target,
        }
    }

    /// Interpolates the transition to an Instant.
    pub fn interpolate(&self, current: &Style, now: Option<Instant>) -> (Style, bool) {
        // The progress of the transition as [0, 1].
        let progress = now
            .unwrap_or(Instant::now())
            .duration_since(self.start_at)
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
                opacity: interp!(target.opacity, current.opacity, progress),
            },
            progress >= 1.,
        )
    }
}
