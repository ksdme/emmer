use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use smithay_client_toolkit::seat::pointer::{CursorIcon, PointerEvent};

use crate::{
    config::{ComputedConfig, Insets},
    dbus::CloseReason,
    notification::{Action, Notification},
    ui::{
        items::stack::{AppCommand, Presentation},
        renderables::{Rect, button, notification},
    },
};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum VisualState {
    Stacked { pos: usize, y: f64 },
    Spread { y: f64 },
    Hidden { y: f64 },
}

/// Represents an action button.
#[derive(Debug)]
pub struct ActionButton {
    action: Action,

    r: button::Renderable,
    style: button::Style,
    transition: Option<button::StyleTransition>,

    bounds: Option<Rect>,
}

/// Represents a logical notification item.
#[derive(Debug)]
pub struct Item {
    config: Arc<ComputedConfig>,

    visual_state: VisualState,
    dismissed: bool,

    notif: Notification,
    notif_r: notification::Renderable,
    notif_style: notification::Style,
    notif_transition: Option<notification::StyleTransition>,
    notif_bounds: Option<Rect>,

    action_buttons: Vec<ActionButton>,
    hovering_button: Option<usize>,
    buttons: bool,

    bounds: Option<Rect>,
}

impl Item {
    pub fn spawn(
        notif: Notification,
        config: Arc<ComputedConfig>,
        presentation: &Presentation,
    ) -> Self {
        let notif_r = notification::Renderable::new(&config, &notif);

        let (w, h) = notif_r.content_size();
        let notif_style = notification::Style {
            x: config.margin.x,
            y: match presentation {
                Presentation::Stack => -config.margin.y,
                Presentation::Spread => config.margin.y - config.spread.gap - h,
            },

            w,
            h,

            outer_opacity: 1.,
            inner_opacity: 1.,
        };

        let action_buttons: Vec<ActionButton> = notif
            .actions()
            .iter()
            .map(|action| ActionButton {
                action: action.clone(),

                r: button::Renderable::new(&config, action.label(), Insets { x: 16., y: 8. }),
                style: button::Style::default(),
                transition: None,

                bounds: None,
            })
            .collect();

        Self {
            config,

            visual_state: VisualState::Hidden { y: 0. },
            dismissed: false,

            notif,
            notif_r,
            notif_style,
            notif_transition: None,
            notif_bounds: None,

            action_buttons,
            hovering_button: None,
            buttons: false,

            bounds: None,
        }
    }

    pub fn id(&self) -> u32 {
        self.notif.id()
    }

    pub fn notification(&self) -> &Notification {
        &self.notif
    }

    pub fn dismissed(&self) -> bool {
        self.dismissed
    }

    /// Marks the current item as dismissed and returns a boolean if a boolean
    /// indicating if the change was accepted.
    pub fn dismiss(&mut self) -> bool {
        if !self.dismissed {
            self.dismissed = true;
            true
        } else {
            false
        }
    }

    pub fn on_hover(&mut self, event: &PointerEvent) -> Vec<AppCommand> {
        // Check if the hover was on the card itself.
        if let Some(bounds) = self.notif_bounds
            && bounds.contains(event.position)
        {
            return vec![AppCommand::SetCursor(CursorIcon::Pointer)];
        }

        // Check if the hover was on the button instead.
        if self.buttons {
            let hit = self.action_buttons.iter_mut().enumerate().find(|el| {
                el.1.bounds
                    .map(|bounds| bounds.contains(event.position))
                    .unwrap_or(false)
            });

            if let Some((current, button)) = hit {
                let mut commands = vec![AppCommand::SetCursor(CursorIcon::Pointer)];

                if self.hovering_button != Some(current) {
                    // Show hover tint on the current button.
                    button.transition = Some(button::StyleTransition::new(
                        Duration::from_millis(100),
                        button::PartialStyle {
                            light: Some(0.025),
                            opacity: Some(1.),
                        },
                        None,
                    ));

                    // Reset tint on the previous button.
                    if let Some(previous) = self.hovering_button
                        && let Some(button) = self.action_buttons.get_mut(previous)
                    {
                        button.transition = Some(button::StyleTransition::new(
                            Duration::from_millis(100),
                            button::PartialStyle {
                                light: Some(0.),
                                opacity: Some(1.),
                            },
                            None,
                        ));
                    }

                    self.hovering_button = Some(current);

                    // Trigger the draw loop only if something has changed here,
                    // otherwise, we will draw on almost every frame.
                    commands.push(AppCommand::Redraw);
                }

                return commands;
            }
        }

        vec![]
    }

    pub fn on_leave(&mut self, _event: &PointerEvent) -> Vec<AppCommand> {
        // TODO: Should we redraw?
        if let Some(id) = self.hovering_button
            && let Some(button) = self.action_buttons.get_mut(id)
        {
            button.transition = Some(button::StyleTransition::new(
                Duration::from_millis(100),
                button::PartialStyle {
                    light: Some(0.),
                    opacity: Some(1.),
                },
                None,
            ));

            self.hovering_button = None;
        }

        vec![]
    }

    /// The left click handler on the item. Returns a list of commands and also a bool
    /// representing if the stack layout should be refreshed.
    pub fn on_left_click(&mut self, event: &PointerEvent) -> (Vec<AppCommand>, bool) {
        // Check the notif.
        if let Some(bounds) = self.notif_bounds
            && bounds.contains(event.position)
        {
            self.buttons = !self.buttons;

            // Again, the true here should trigger a visual state update and a
            // redraw automatically.
            return (vec![], true);
        }

        // Check buttons.
        if self.buttons && !self.dismissed {
            let action = self
                .action_buttons
                .iter()
                .find(|button| {
                    button
                        .bounds
                        .is_some_and(|bounds| bounds.contains(event.position))
                })
                .map(|button| button.action.key().to_string());

            if let Some(action) = action {
                // A close action will be sent to the client after an activation token
                // is generated. But, visually, we mark it for dismissal right away.
                self.dismiss();

                return (vec![AppCommand::NotifyAction(self.id(), action)], true);
            }
        }

        (vec![], false)
    }

    pub fn on_right_click(&mut self, _event: &PointerEvent) -> (Vec<AppCommand>, bool) {
        if self.dismiss() {
            // Again, the true here should trigger a visual state update and a
            // redraw automatically.
            (
                vec![AppCommand::NotifyClosed(self.id(), CloseReason::Manual)],
                true,
            )
        } else {
            (vec![], false)
        }
    }

    /// Updates the transitions and the style of the item as per the visual state.
    /// Returns the y position that the next visual element can start at if it does not
    /// want to overlap with the current item.
    // TODO: Transtions should not be applied if one towards the same target is underway.
    pub fn set_visual_state(&mut self, visual_state: VisualState, now: Instant) -> f64 {
        // The natural duration of a transition.
        let duration = Duration::from_millis(200);
        let fast_duration = Duration::from_millis(125);

        let (notif_w, notif_h) = self.notif_r.content_size();
        let (notif_transition, buttons_transition, y) = match visual_state {
            VisualState::Stacked { pos, y } if pos == 0 => {
                let notif_target = notification::Style {
                    x: self.config.margin.x,
                    y,

                    w: notif_w,
                    h: notif_h,

                    outer_opacity: if self.dismissed { 0. } else { 1. },
                    inner_opacity: if self.dismissed { 0. } else { 1. },
                };

                let buttons_target = button::Style {
                    light: 0.,
                    opacity: 0.,
                };

                (
                    notification::StyleTransition::new(duration, notif_target.into(), Some(now)),
                    button::StyleTransition::new(fast_duration, buttons_target.into(), Some(now)),
                    y + notif_h,
                )
            }

            VisualState::Stacked { pos, y } => {
                let h = notif_h.min(y - self.config.margin.y);
                let notif_target = notification::PartialStyle {
                    x: Some(self.config.margin.x + (pos as f64) * self.config.stack.inset),
                    y: Some(y + self.config.stack.peek - h),

                    w: Some(self.config.width - 2. * (pos as f64) * self.config.stack.inset),
                    h: Some(h),

                    outer_opacity: Some(if self.dismissed { 0. } else { 1. }),
                    inner_opacity: Some(0.),
                };

                let buttons_target = button::Style {
                    light: 0.,
                    opacity: 0.,
                };

                (
                    notification::StyleTransition::new(duration, notif_target, Some(now)),
                    button::StyleTransition::new(fast_duration, buttons_target.into(), Some(now)),
                    y + self.config.stack.peek,
                )
            }

            VisualState::Spread { y } => {
                let notif_target = notification::Style {
                    x: self.config.margin.x,
                    y: if self.dismissed { y - notif_h } else { y },

                    w: notif_w,
                    h: notif_h,

                    outer_opacity: if self.dismissed { 0. } else { 1. },
                    inner_opacity: if self.dismissed { 0. } else { 1. },
                };

                let buttons_target = if self.buttons && !self.dismissed {
                    button::Style {
                        light: 0.,
                        opacity: 1.,
                    }
                } else {
                    button::Style {
                        light: 0.,
                        opacity: 0.,
                    }
                };

                let buttons_h = if self.buttons {
                    self.action_buttons
                        .first()
                        .map(|button| button.r.content_size())
                        .map(|(_, y)| 8. + y)
                        .unwrap_or_default()
                } else {
                    0.
                };

                (
                    notification::StyleTransition::new(duration, notif_target.into(), Some(now)),
                    button::StyleTransition::new(fast_duration, buttons_target.into(), Some(now)),
                    y + notif_h + buttons_h + self.config.spread.gap,
                )
            }

            VisualState::Hidden { y } => {
                let notif_target = notification::Style {
                    x: self.config.margin.x,
                    y: if self.dismissed { y - notif_h } else { y },

                    w: notif_w,
                    h: notif_h,

                    outer_opacity: 0.,
                    inner_opacity: 0.,
                };

                let buttons_target = button::Style {
                    light: 0.,
                    opacity: 0.,
                };

                (
                    notification::StyleTransition::new(duration, notif_target.into(), Some(now)),
                    button::StyleTransition::new(fast_duration, buttons_target.into(), Some(now)),
                    y + self.config.stack.peek,
                )
            }
        };

        self.visual_state = visual_state;
        self.notif_transition = Some(notif_transition);
        for button in self.action_buttons.iter_mut() {
            button.transition = Some(buttons_transition.clone());
        }

        y
    }

    /// Progresses all the transition attached to the item and returns a boolean
    /// indicating if all the transitions have completed.
    pub fn tick(&mut self, now: &Instant) -> bool {
        // Progress the notification.
        let notif_complete = if let Some(notif_t) = self.notif_transition.as_mut() {
            let (style, complete) = notif_t.interpolate(&self.notif_style, now);

            self.notif_style = style;
            if complete {
                self.notif_transition = None;
            }

            complete
        } else {
            true
        };

        // Progress the action buttons.
        let mut buttons_complete = true;
        for button in self.action_buttons.iter_mut() {
            buttons_complete &= if let Some(button_t) = button.transition.as_mut() {
                let (style, complete) = button_t.interpolate(&button.style, now);

                button.style = style;
                if complete {
                    button.transition = None;
                }

                complete
            } else {
                true
            };
        }

        notif_complete & buttons_complete
    }

    /// Renders the current item to a cairo canvas and returns its rect bounds.
    pub fn render(&mut self, cr: &cairo::Context) -> Result<Option<Rect>> {
        // If the card is not visible, do not even try rendering.
        let notif_bounds = if self.notif_style.outer_opacity > 0. {
            let notif_bounds = self
                .notif_r
                .render(cr, &self.notif_style)
                .context("Could not render notification")?;
            self.notif_bounds = Some(notif_bounds);

            #[cfg(debug_assertions)]
            if self.config.debug_mode {
                cr.new_path();
                cr.set_source_rgba(0., 0., 255., 0.5);
                cr.rectangle(
                    notif_bounds.x1,
                    notif_bounds.y1,
                    notif_bounds.w(),
                    notif_bounds.h(),
                );
                let _ = cr.stroke();
            }

            Some(notif_bounds)
        } else {
            None
        };

        // If the card was not rendered or if the button is not visible, do not
        // try even try rendering.
        let bounds = if let Some(notif_bounds) = notif_bounds
            && let Some(first) = self.action_buttons.first()
            && first.style.opacity > 0.
        {
            let gap = 8.;
            let mut bounds = notif_bounds;

            let mut x = notif_bounds.x1;
            let y = notif_bounds.y2 + gap;

            for button in self.action_buttons.iter_mut() {
                let button_bounds = button
                    .r
                    .render(cr, &button.style, x, y)
                    .context("Could not draw")?;
                button.bounds = Some(button_bounds);

                x = button_bounds.x2 + gap;
                bounds.y2 = y + button_bounds.h();
            }

            Some(bounds)
        } else {
            notif_bounds
        };

        self.bounds = bounds;
        Ok(bounds)
    }

    pub fn bounds(&self) -> Option<&Rect> {
        self.bounds.as_ref()
    }
}
