use std::fmt::Write;

use pango::FontDescription;

use crate::{
    config::{
        GlobalConfig,
        kdl::{Setting, settings},
        theme::TextConfig,
    },
    ui::renderables::Color,
};

settings! {
    /// The settings of the global config that a rule can override.
    #[derive(Default)]
    pub struct RuleConfig {
        /// The notification cards.
        pub notification: RuleNotificationConfig,
    }

    /// The settings of the notification cards that a rule can override.
    #[derive(Default)]
    pub struct RuleNotificationConfig {
        /// The summary line.
        pub title: TextConfig,

        /// The action buttons below the notification.
        pub actions: RuleControlConfig,
    }

    /// The settings of a clickable control that a rule can override.
    #[derive(Default)]
    pub struct RuleControlConfig {
        /// Overrides the background color.
        pub background: Option<Color>,

        /// Overrides the border color.
        pub border_color: Option<Color>,

        /// Overrides the font of the labels.
        pub font: Option<FontDescription>,

        /// Overrides the color of the labels.
        pub color: Option<Color>,
    }
}

/// Documents the rules in the kdl config, which mirror the global config.
pub const DOCS: &str = r##"// The rules override the settings above for the notifications from an app.
// Every rule that matches applies in order, so the later ones win.
// rule app-name="Slack" {
//     notification {
//         title {
//             color "#e01e5a"
//         }
//     }
// }
"##;

/// Overrides the global config for the notifications it matches.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub app_name: String,
    pub config: RuleConfig,
}

#[derive(knus::Decode)]
pub struct RulePatch {
    #[knus(property(name = "app-name"))]
    app_name: String,

    #[knus(flatten(child))]
    config: RuleConfigPatch,
}

impl From<RulePatch> for Rule {
    fn from(patch: RulePatch) -> Self {
        let mut config = RuleConfig::default();
        config.apply(patch.config);
        Self {
            app_name: patch.app_name,
            config,
        }
    }
}

impl Rule {
    pub fn matches(&self, app_name: &str) -> bool {
        self.app_name == app_name
    }

    /// Overrides the settings of the config that are set in the rule.
    pub fn apply_to(&self, config: &mut GlobalConfig) {
        let notification = &mut config.notification;
        let over = &self.config.notification;

        let title = &mut notification.title;
        set(&mut title.font, &over.title.font);
        set(&mut title.color, &over.title.color);

        let actions = &mut notification.actions;
        set(&mut actions.background, &over.actions.background);
        set(&mut actions.border_color, &over.actions.border_color);
        set(&mut actions.font, &over.actions.font);
        set(&mut actions.color, &over.actions.color);
    }

    /// Writes the rule as a kdl node, with the docs of each field as comments.
    pub fn write(&self, out: &mut String) {
        let _ = writeln!(out, "rule app-name={:?} {{", self.app_name);
        self.config.write_fields(&RuleConfig::default(), out, 1);
        out.push_str("}\n");
    }
}

// Overrides the value only if the rule sets it.
fn set<T: Clone>(value: &mut Option<T>, over: &Option<T>) {
    if over.is_some() {
        value.clone_from(over);
    }
}
