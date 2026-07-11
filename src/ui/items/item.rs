use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use smithay_client_toolkit::seat::pointer::{CursorIcon, PointerEvent};

use crate::{
    config::{ComputedConfig, Insets},
    dbus::{
        notification::{Action, Notification},
        service::CloseReason,
    },
    ui::{
        items::stack::{AppCommand, Presentation},
        renderables::{Rect, button, notification},
    },
};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum VisualState {
    Stacked { pos: usize, y: f64 },
    StackedHidden { y: f64 },
    Spread { y: f64 },
    SpreadHidden { y: f64 },
}

/// Represents an action button.
#[derive(Debug)]
pub struct ActionButton {
    action: Action,

    r: button::Renderable,
    style: button::Style,
    transition: Option<button::StyleTransition>,

    // Since the layout of these buttons are fixed, we calculate
    // them ahead of time and store it relative to the bottom left corner
    // of the notification card.
    base_x: f64,
    base_y: f64,

    bounds: Option<Rect>,
}

/// Represents a logical notification item.
#[derive(Debug)]
pub struct Item {
    config: Arc<ComputedConfig>,

    // Instead of holding onto the notification object we move the fields here
    // so we can prevent duplicating the image data or having to use cairo's
    // unsafe methods.
    id: u32,
    expires_at: Option<Instant>,

    visual_state: VisualState,
    dismissed: bool,

    notif_r: notification::Renderable,
    notif_style: notification::Style,
    notif_transition: Option<notification::StyleTransition>,
    notif_bounds: Option<Rect>,

    implicit_action: Option<Action>,
    action_buttons: Option<Vec<ActionButton>>,
    hovering_button: Option<usize>,
    buttons_visible: bool,

    bounds: Option<Rect>,
}

impl Item {
    pub fn spawn(
        notif: Notification,
        config: Arc<ComputedConfig>,
        presentation: &Presentation,
    ) -> Result<Self> {
        let notif_r = notification::Renderable::new(
            &config,
            notif.title.as_deref(),
            notif.body.as_deref(),
            notif.image,
        )
        .context("Could not initialize")?;

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

        let (implicit_action, action_buttons) = match notif.actions.as_slice() {
            [] => (None, None),

            // If there is only one action, we will treat it as implicit action.
            [first] => (Some(first.clone()), None),

            // If there are more than one actions, we turn all of them into buttons.
            actions => {
                let gap = 8.;

                // Assumes that buttons are only shown in spread mode which means the available
                // width for the buttons to spread equals to that of the notification card.
                let buttons = actions
                    .iter()
                    .scan((0., 8.), |(x, y), action| {
                        let r = button::Renderable::new(
                            &config,
                            action.label(),
                            Insets { x: 16., y: 8. },
                            Some(w as i32),
                        );

                        let (b_w, b_h) = r.content_size();
                        if *x + b_w > w {
                            *x = 0.;
                            *y += b_h + gap;
                        }

                        let button = ActionButton {
                            action: action.clone(),

                            r,
                            style: button::Style::default(),
                            transition: None,

                            base_x: *x,
                            base_y: *y,

                            bounds: None,
                        };
                        *x += b_w + gap;

                        Some(button)
                    })
                    .collect::<Vec<ActionButton>>();

                (None, Some(buttons))
            }
        };

        Ok(Self {
            config,

            id: notif.id,
            expires_at: notif.expires_at,

            visual_state: VisualState::StackedHidden { y: 0. },
            dismissed: false,

            notif_r,
            notif_style,
            notif_transition: None,
            notif_bounds: None,

            implicit_action,
            action_buttons,
            hovering_button: None,
            buttons_visible: false,

            bounds: None,
        })
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|at| Instant::now() >= at)
    }

    pub fn was_dismissed(&self) -> bool {
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
            // If the notif is interactable at all.
            if self.implicit_action.is_some() || self.action_buttons.is_some() {
                return vec![AppCommand::SetCursor(CursorIcon::Pointer)];
            }
        }

        // Check if the hover was on the button instead.
        if self.buttons_visible {
            let hit = self.action_buttons.as_mut().and_then(|buttons| {
                buttons.iter_mut().enumerate().find(|el| {
                    el.1.bounds
                        .map(|bounds| bounds.contains(event.position))
                        .unwrap_or(false)
                })
            });

            if let Some((current, button)) = hit {
                let mut commands = vec![AppCommand::SetCursor(CursorIcon::Pointer)];

                if self.hovering_button != Some(current) {
                    // Show hover tint on the current button.
                    button.transition = Some(button::StyleTransition::new(
                        Duration::from_millis(100),
                        button::PartialStyle {
                            light: Some(0.03),
                            opacity: Some(1.),
                        },
                        None,
                    ));

                    // Reset tint on the previous button.
                    if let Some(previous) = self.hovering_button
                        && let Some(buttons) = self.action_buttons.as_mut()
                        && let Some(button) = buttons.get_mut(previous)
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
            && let Some(buttons) = self.action_buttons.as_mut()
            && let Some(button) = buttons.get_mut(id)
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
        if !self.dismissed
            && let Some(bounds) = self.notif_bounds
            && bounds.contains(event.position)
        {
            // Issue the implicit action if possible.
            if let Some(action) = self
                .implicit_action
                .as_ref()
                .map(|action| action.key().to_string())
            {
                // An action should dismiss the item. NotifyAction should send the NotifyClosed
                // action itself.
                self.dismiss();

                return (vec![AppCommand::NotifyAction(self.id(), action)], true);
            }

            // Toggle the buttons.
            if let Some(_) = self.action_buttons {
                self.buttons_visible = !self.buttons_visible;

                // Again, the true here should trigger a visual state update and a
                // redraw automatically.
                return (vec![], true);
            }

            return (vec![], false);
        }

        // Check buttons.
        if !self.dismissed && self.buttons_visible {
            let action = self.action_buttons.as_ref().and_then(|buttons| {
                buttons
                    .iter()
                    .find(|button| {
                        button
                            .bounds
                            .is_some_and(|bounds| bounds.contains(event.position))
                    })
                    .map(|button| button.action.key().to_string())
            });

            if let Some(action) = action {
                // An action should dismiss the item. NotifyAction should send the NotifyClosed
                // action itself.
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
            VisualState::Stacked { pos: 0, y } => {
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
                let pos = pos as f64;

                let h = notif_h.min(y - self.config.margin.y);
                let notif_target = notification::Style {
                    x: self.config.margin.x + pos * self.config.stack.inset,
                    y: y + self.config.stack.peek - h,

                    w: self.config.width - 2. * pos * self.config.stack.inset,
                    h,

                    outer_opacity: if self.dismissed { 0. } else { 1. },
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

            VisualState::StackedHidden { y } => {
                let pos = self.config.stack.max_count as f64;

                let h = notif_h.min(y - self.config.margin.y);
                let notif_target = notification::Style {
                    x: self.config.margin.x + pos * self.config.stack.inset,
                    y: y + self.config.stack.peek - h,

                    w: self.config.width - 2. * pos * self.config.stack.inset,
                    h,

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
                    y,
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

                let buttons_target = if self.buttons_visible && !self.dismissed {
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

                let buttons_h = if self.buttons_visible {
                    self.action_buttons
                        .as_ref()
                        // Assuming that the buttons are drawn and laid out in order meaning
                        // the bottom most button must be the last item in the list.
                        .and_then(|buttons| buttons.last())
                        .map(|button| {
                            let (_, h) = button.r.content_size();
                            button.base_y + h
                        })
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

            VisualState::SpreadHidden { y } => {
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
        if let Some(buttons) = self.action_buttons.as_mut() {
            for button in buttons.iter_mut() {
                button.transition = Some(buttons_transition.clone());
            }
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
        if let Some(buttons) = self.action_buttons.as_mut() {
            for button in buttons.iter_mut() {
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

        // If the card was not rendered or if the buttons are not visible, do not
        // even try rendering them.
        let bounds = if let Some(notif_bounds) = notif_bounds
            && let Some(buttons) = self.action_buttons.as_mut()
            && let Some(first) = buttons.first()
            && first.style.opacity > 0.
        {
            let mut bounds = notif_bounds;

            for button in buttons.iter_mut() {
                let button_bounds = button
                    .r
                    .render(
                        cr,
                        &button.style,
                        notif_bounds.x1 + button.base_x,
                        notif_bounds.y2 + button.base_y,
                    )
                    .context("Could not draw")?;

                button.bounds = Some(button_bounds);

                // Extending the bounds that notification created to include the
                // buttons from here.
                bounds.y2 = button_bounds.y2;
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
