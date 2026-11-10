use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use smithay_client_toolkit::seat::pointer::{CursorIcon, PointerEvent};

use crate::{
    config::ComputedConfig,
    dbus::{
        notification::{Action, Notification},
        service::CloseReason,
    },
    ui::{
        anchors::{Anchor, VerticalAnchor},
        items::{
            layout::{self, Layout},
            stack::{AppCommand, Time},
        },
        renderables::{Animated, Easing, Rect, Renderable, Timing, button, notification},
    },
};

/// Represents an action button.
#[derive(Debug)]
pub struct ActionButton {
    action: Action,

    r: button::Button,
    // The hover tint, which is interaction state and is never touched by the layout.
    light: Animated<f64>,

    // Since the layout of these buttons are fixed, we calculate
    // them ahead of time and store it relative to the bottom left corner
    // of the notification card.
    base_x: f64,
    base_y: f64,
}

impl ActionButton {
    fn rect_below(&self, card: &Rect) -> Rect {
        Rect::from_origin(
            card.x1 + self.base_x,
            card.y2 + self.base_y,
            self.r.measure(),
        )
    }
}

/// Represents a logical notification item.
#[derive(Debug)]
pub struct Item {
    config: Rc<ComputedConfig>,

    // Instead of holding onto the notification object we move the fields here
    // so we can prevent duplicating the image data or having to use cairo's
    // unsafe methods.
    id: u32,
    expires_at: Option<Time>,

    dismissed: bool,

    notif_r: notification::Notification,
    // The layout state of the item, which is only ever driven by set_target.
    visual: layout::AnimatedTarget,

    implicit_action: Option<Action>,
    action_buttons: Option<Vec<ActionButton>>,
    hovering_button: Option<usize>,
    buttons_visible: bool,
}

impl Item {
    pub fn spawn(
        now: Time,
        notif: Notification,
        config: Rc<ComputedConfig>,
        layout: &Layout,
    ) -> Result<Self> {
        let mut notif_r = notification::Notification::new(
            config.clone(),
            Some(&notif.app_name),
            notif.image,
            notif.title.as_deref(),
            notif.body.as_deref(),
            Some(notif.created_at.format("%-I:%M %P").to_string().as_str()),
        )
        .context("Could not initialize")?;
        notif_r.set_clip_anchor(clip_anchor(layout.anchor));

        let size = notif_r.measure();
        let w = size.w;

        let visual = layout::Target {
            rect: layout.spawn(size),
            style: notification::Style {
                outer_opacity: 1.,
                inner_opacity: 1.,
            },
            buttons: layout::ButtonsTarget { opacity: 0. },
        };

        let (implicit_action, action_buttons) = match notif.actions.as_slice() {
            [] => (None, None),

            // If there is only one action, we will treat it as implicit action.
            [first] => (Some(first.clone()), None),

            // If there are more than one actions, we turn all of them into buttons.
            actions => {
                let gap = config.theme.button_gap;

                // Assumes that buttons are only shown in spread mode which means the available
                // width for the buttons to spread equals to that of the notification card.
                let buttons = actions
                    .iter()
                    .scan((0., gap), |(x, y), action| {
                        let r = button::Button::new(config.clone(), action.label(), Some(w as i32));

                        let b_size = r.measure();
                        let (b_w, b_h) = (b_size.w, b_size.h);
                        if *x + b_w > w {
                            *x = 0.;
                            *y += b_h + gap;
                        }

                        let button = ActionButton {
                            action: action.clone(),

                            r,
                            light: Animated::new(0.),

                            base_x: *x,
                            base_y: *y,
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
            expires_at: notif.expiry.map(|duration| now + duration),

            dismissed: false,

            notif_r,
            visual: layout::AnimatedTarget::new(visual),

            implicit_action,
            action_buttons,
            hovering_button: None,
            buttons_visible: false,
        })
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn is_expired(&self, now: Time) -> bool {
        self.expires_at.is_some_and(|at| now >= at)
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

    fn reset_hovering_button(&mut self) -> bool {
        if let Some(current) = self.hovering_button
            && let Some(buttons) = self.action_buttons.as_mut()
            && let Some(button) = buttons.get_mut(current)
        {
            // Rest tint on the current button.
            button.light.animate_to(
                0.,
                Timing::new(Duration::from_millis(100), Easing::EaseOutCubic),
                Instant::now(),
            );

            self.hovering_button = None;

            true
        } else {
            false
        }
    }

    // The rect the card occupies in the current layout, if it is interactable.
    fn notif_target(&self) -> Option<Rect> {
        let target = self.visual.target();
        (target.style.outer_opacity > 0.).then_some(target.rect)
    }

    // The rects the buttons occupy in the current layout, if they are interactable.
    fn button_targets(&self) -> impl Iterator<Item = Rect> + '_ {
        let card = self
            .notif_target()
            .filter(|_| self.visual.buttons.target().opacity > 0.);

        card.into_iter().flat_map(|card| {
            self.action_buttons
                .iter()
                .flatten()
                .map(move |button| button.rect_below(&card))
        })
    }

    fn card_hit(&self, at: (f64, f64)) -> bool {
        self.notif_target()
            .is_some_and(|target| target.contains(at))
    }

    // Returns the index of the button at (x, y), if any.
    fn button_at(&self, at: (f64, f64)) -> Option<usize> {
        self.button_targets().position(|target| target.contains(at))
    }

    pub fn on_hover(&mut self, event: &PointerEvent) -> Vec<AppCommand> {
        // Check if the hover was on the card itself.
        if self.card_hit(event.position) {
            let mut commands = vec![];

            // Since the hover target has now changed to the card, if there was already
            // a button that was being hovered, reset it.
            if self.reset_hovering_button() {
                commands.push(AppCommand::Redraw);
            }

            // If the notif is interactable at all.
            if self.implicit_action.is_some() || self.action_buttons.is_some() {
                commands.push(AppCommand::SetCursor(CursorIcon::Pointer));
            }

            return commands;
        }

        // Check if the hover was on the button instead.
        if let Some(current) = self.button_at(event.position) {
            let mut commands = vec![AppCommand::SetCursor(CursorIcon::Pointer)];

            if self.hovering_button != Some(current) {
                // Clear the previous button.
                let _ = self.reset_hovering_button();

                // Show hover tint on the current button.
                if let Some(button) = self
                    .action_buttons
                    .as_mut()
                    .and_then(|buttons| buttons.get_mut(current))
                {
                    button.light.animate_to(
                        0.03,
                        Timing::new(Duration::from_millis(100), Easing::EaseOutCubic),
                        Instant::now(),
                    );
                }

                // Track the current button.
                self.hovering_button = Some(current);

                // Trigger the draw loop only if something has changed here,
                // otherwise, we will draw on almost every frame.
                commands.push(AppCommand::Redraw);
            }

            return commands;
        }

        vec![]
    }

    pub fn on_leave(&mut self, _event: &PointerEvent) -> Vec<AppCommand> {
        if self.reset_hovering_button() {
            vec![AppCommand::Redraw]
        } else {
            vec![]
        }
    }

    /// The left click handler on the item. Returns a list of commands and also a bool
    /// representing if the stack layout should be refreshed.
    pub fn on_left_click(&mut self, event: &PointerEvent) -> (Vec<AppCommand>, bool) {
        // Check the notif.
        if !self.dismissed && self.card_hit(event.position) {
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
            if self.action_buttons.is_some() {
                self.buttons_visible = !self.buttons_visible;

                // Again, the true here should trigger a visual state update and a
                // redraw automatically.
                return (vec![], true);
            }

            return (vec![], false);
        }

        // Check buttons.
        if !self.dismissed {
            let action = self.button_at(event.position).and_then(|current| {
                self.action_buttons
                    .as_ref()
                    .and_then(|buttons| buttons.get(current))
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

    /// Progresses all the transition attached to the item and returns a boolean
    /// indicating if all the transitions have completed.
    pub fn tick(&mut self, now: Instant) -> bool {
        let mut complete = self.visual.tick(now);
        for button in self.action_buttons.iter_mut().flatten() {
            complete &= button.light.tick(now);
        }

        complete
    }

    /// Renders the current item to a cairo canvas and returns the rect it was
    /// drawn in.
    pub fn render(&self, cr: &cairo::Context) -> Result<Option<Rect>> {
        let visual = self.visual.get();

        // If the card is not visible, do not even try rendering.
        if visual.style.outer_opacity <= 0. {
            return Ok(None);
        }

        let notif_rect = visual.rect;
        self.notif_r
            .paint(cr, notif_rect, &visual.style)
            .context("Could not render notification")?;

        #[cfg(debug_assertions)]
        if self.config.debug_mode {
            cr.new_path();
            cr.set_source_rgba(0., 0., 255., 0.5);
            cr.rectangle(notif_rect.x1, notif_rect.y1, notif_rect.w(), notif_rect.h());
            let _ = cr.stroke();
        }

        let mut bounds = notif_rect;

        // If the buttons are not visible, do not even try rendering them.
        let opacity = visual.buttons.opacity;
        if let Some(buttons) = self.action_buttons.as_ref()
            && opacity > 0.
        {
            for button in buttons {
                let rect = button.rect_below(&notif_rect);
                let style = button::Style {
                    light: button.light.get(),
                    opacity,
                };
                button.r.paint(cr, rect, &style).context("Could not draw")?;

                bounds = bounds.union(&rect);
            }
        }

        Ok(Some(bounds))
    }
}

impl Item {
    /// The rect the item occupies in the current layout, if it is interactable.
    pub fn target(&self) -> Option<Rect> {
        self.notif_target().map(|card| {
            self.button_targets()
                .fold(card, |target, button| target.union(&button))
        })
    }

    /// Returns what the layout needs to know about the item.
    pub fn layout_input(&self) -> layout::Input {
        // The full height of the buttons block, if it should be visible.
        let buttons_h = if self.buttons_visible {
            self.action_buttons
                .as_ref()
                // Assuming that the buttons are drawn and laid out in order meaning
                // the bottom most button must be the last item in the list.
                .and_then(|buttons| buttons.last())
                .map(|button| button.base_y + button.r.measure().h)
                .unwrap_or(0.)
        } else {
            0.
        };

        layout::Input {
            size: self.notif_r.measure(),
            buttons_h,
            dismissed: self.dismissed,
        }
    }

    pub fn set_anchor(&mut self, anchor: &Anchor) {
        self.notif_r.set_clip_anchor(clip_anchor(anchor));
    }

    /// Moves the item towards a new layout target. Only the parts of the target
    /// that changed are transitioned, so that the ones underway are not restarted.
    pub fn set_target(&mut self, target: layout::Target, now: Instant) {
        self.visual.animate_to(
            target,
            Timing::new(Duration::from_millis(200), Easing::EaseOutCubic),
            now,
        );
    }
}

// The edge that the contents of a clipped card stick to.
fn clip_anchor(anchor: &Anchor) -> VerticalAnchor {
    match anchor.vertical {
        VerticalAnchor::Top => VerticalAnchor::Bottom,
        VerticalAnchor::Bottom => VerticalAnchor::Top,
    }
}
