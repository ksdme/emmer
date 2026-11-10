use crate::{
    config::{Config, Insets, SpreadConfig, StackConfig, ThemeConfig},
    ui::anchors::{Anchor, HorizontalAnchor, VerticalAnchor},
};

/// Represents a prepared configuration.
#[derive(Debug)]
pub struct ComputedConfig {
    pub debug_mode: bool,

    pub width: f64,
    pub margin: Insets,

    pub anchors: (HorizontalAnchor, VerticalAnchor),

    pub stack: StackConfig,
    pub spread: SpreadConfig,

    pub theme: ThemeConfig,
    pub font_map: pango::FontMap,
}

impl ComputedConfig {
    pub fn new(debug_mode: bool, config: Config) -> Self {
        Self {
            debug_mode,

            anchors: (HorizontalAnchor::Right, VerticalAnchor::Bottom),

            width: config.width,
            margin: config.margin,

            stack: config.stack,
            spread: config.spread,

            theme: config.theme,
            font_map: pangocairo::FontMap::default(),
        }
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
