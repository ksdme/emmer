use pango::FontDescription;

use crate::{
    config::{Config, ControlConfig, GlobalConfig, Insets, SpreadConfig, StackConfig, rules::Rule},
    ui::{
        anchors::{Anchor, HorizontalAnchor, VerticalAnchor},
        renderables::Color,
    },
};

/// Represents a prepared configuration.
#[derive(Debug)]
pub struct ComputedConfig {
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    pub debug_mode: bool,

    pub width: f64,
    pub margin: Insets,

    pub anchors: (HorizontalAnchor, VerticalAnchor),

    pub stack: StackConfig,
    pub spread: SpreadConfig,

    pub theme: Theme,
    pub font_map: pango::FontMap,

    // Kept around to derive the themes of the apps that the rules match.
    global: GlobalConfig,
    rules: Vec<Rule>,
}

impl ComputedConfig {
    pub fn new(debug_mode: bool, Config { global, rules }: Config) -> Self {
        Self {
            debug_mode,

            anchors: (global.anchor.horizontal, global.anchor.vertical),

            theme: Theme::new(&global),

            width: global.width,
            margin: global.margin.clone(),

            stack: global.stack.clone(),
            spread: global.spread.clone(),

            font_map: pangocairo::FontMap::default(),

            global,
            rules,
        }
    }

    /// The theme of the notifications from an app, with the rules that match
    /// it applied in order.
    pub fn theme_for(&self, app_name: &str) -> Theme {
        let mut global = self.global.clone();
        for rule in self.rules.iter().filter(|rule| rule.matches(app_name)) {
            rule.apply_to(&mut global);
        }

        Theme::new(&global)
    }

    pub fn anchor(&self, l_w: f64, l_h: f64) -> Anchor {
        Anchor {
            horizontal: self.anchors.0,
            vertical: self.anchors.1,

            layer_width: l_w,
            layer_height: l_h,
        }
    }
}

/// The look of a box shaped surface.
#[derive(Debug, Clone)]
pub struct Surface {
    pub background: Color,
    pub border_width: f64,
    pub border_color: Color,
    pub radius: f64,
    pub padding: Insets,
}

/// The look of a piece of text.
#[derive(Debug, Clone)]
pub struct Text {
    pub font: FontDescription,
    pub color: Color,
}

/// The style tokens of the renderables, with the fallbacks of the config
/// resolved.
#[derive(Debug)]
pub struct Theme {
    pub notification: Surface,
    pub title: Text,
    pub body: Text,
    pub footer: Text,
    pub image_radius: f64,
    pub image_gap: f64,
    pub title_gap: f64,
    pub footer_gap: f64,

    pub button: Surface,
    pub button_label: Text,
    pub button_gap: f64,

    pub bar: Surface,
    pub bar_label: Text,
    pub bar_gap: f64,
}

impl Theme {
    fn new(config: &GlobalConfig) -> Self {
        let colors = &config.colors;

        let text = |font: &Option<FontDescription>, color: Option<Color>, fallback: Color| Text {
            font: font.clone().unwrap_or_else(|| config.font.clone()),
            color: color.unwrap_or(fallback),
        };

        let control = |control: &ControlConfig| Surface {
            background: control.background.unwrap_or(colors.background),
            border_width: control.border_width,
            border_color: control.border_color.unwrap_or(colors.border),
            radius: control.radius,
            padding: control.padding.clone(),
        };

        let n = &config.notification;
        let (actions, bar) = (&n.actions, &config.bar);
        Self {
            notification: Surface {
                background: n.background.unwrap_or(colors.background),
                border_width: n.border_width,
                border_color: n.border_color.unwrap_or(colors.border),
                radius: n.radius,
                padding: n.padding.clone(),
            },
            title: text(&n.title.font, n.title.color, colors.text),
            body: text(&n.body.font, n.body.color, colors.text),
            footer: text(&n.footer.font, n.footer.color, colors.muted),
            image_radius: n.image.radius,
            image_gap: n.image.gap,
            title_gap: n.title.gap,
            footer_gap: n.footer.gap,

            button: control(actions),
            button_label: text(&actions.font, actions.color, colors.text),
            button_gap: actions.gap,

            bar: control(bar),
            bar_label: text(&bar.font, bar.color, colors.text),
            bar_gap: bar.gap,
        }
    }
}

#[cfg(test)]
mod tests {
    use pango::Weight;

    use super::*;

    #[test]
    fn parts_fall_back_to_the_shared_look() {
        let mut config = GlobalConfig::default();
        config.colors.text = Color::from_rgba_u8(0, 0, 0, 1.);
        config.bar.background = Some(Color::from_rgba_u8(255, 0, 0, 1.));
        config.bar.font = Some(FontDescription::from_string("Inter 12"));

        let theme = Theme::new(&config);
        assert_eq!(theme.title.font.weight(), Weight::Bold);
        assert_eq!(theme.body.font, config.font);
        assert_eq!(theme.bar_label.font.to_str(), "Inter 12");
        assert_eq!(theme.body.color.to_hex(), "#000000");
        assert_eq!(theme.footer.color, config.colors.muted);
        assert_eq!(theme.button.background, config.colors.background);
        assert_eq!(theme.bar.background.to_hex(), "#ff0000");
    }

    #[test]
    fn matching_rules_override_the_theme_in_order() {
        let config = Config::from_kdl(
            "config.kdl",
            r##"
            colors { text "#ffffff"; }
            notification { title { font "Inter 9"; }; }

            rule app-name="Slack" { notification { title { color "#ff0000"; }; }; }
            rule app-name="Firefox" { notification { title { color "#00ff00"; }; }; }
            rule app-name="Slack" { notification { title { color "#0000ff"; }; actions { color "#00ffff"; }; }; }
            "##,
        )
        .unwrap();
        let config = ComputedConfig::new(false, config);

        let slack = config.theme_for("Slack");
        assert_eq!(slack.title.color.to_hex(), "#0000ff");
        assert_eq!(slack.title.font.to_str(), "Inter 9");
        assert_eq!(slack.body.color.to_hex(), "#ffffff");
        assert_eq!(slack.button_label.color.to_hex(), "#00ffff");
        assert_eq!(slack.button.background, config.theme.button.background);

        assert_eq!(config.theme_for("Firefox").title.color.to_hex(), "#00ff00");
        assert_eq!(config.theme_for("slack").title.color.to_hex(), "#ffffff");
    }
}
