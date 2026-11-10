use std::{
    env, fs,
    path::{Path, PathBuf},
};

use miette::IntoDiagnostic;
use struct_patch::Patch;

use crate::config::{Config, ConfigPatch, kdl::Fields};

impl Config {
    /// Reads the configuration from a kdl file.
    pub fn load(path: &Path) -> miette::Result<Self> {
        let contents = fs::read_to_string(path).into_diagnostic()?;
        Self::from_kdl(&path.display().to_string(), &contents)
    }

    /// Parses the configuration from a kdl (v1) document, which only needs to
    /// contain the values that differ from the defaults.
    pub fn from_kdl(name: &str, contents: &str) -> miette::Result<Self> {
        let patch: ConfigPatch = knus::parse(name, contents)?;
        let mut config = Self::default();
        config.apply(patch);
        Ok(config)
    }

    /// Returns the configuration as a kdl (v1) document, with the docs of each
    /// field as comments.
    pub fn to_kdl(&self) -> String {
        let mut out = String::new();
        self.write_fields(&mut out, 0);
        out
    }
}

/// Returns the path of the config file in the XDG config directory.
pub fn default_path() -> Option<PathBuf> {
    let base = env::var_os("XDG_CONFIG_HOME")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;

    Some(base.join("emmer").join("config.kdl"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(kdl: &str) -> miette::Result<Config> {
        Config::from_kdl("config.kdl", kdl)
    }

    #[test]
    fn default_config_round_trips() {
        let kdl = Config::default().to_kdl();
        let config = parse(&kdl).unwrap();
        assert_eq!(config.to_kdl(), kdl);
    }

    #[test]
    fn default_config_has_the_field_docs_as_comments() {
        let kdl = Config::default().to_kdl();
        assert!(kdl.starts_with("// The width of a notification card.\nwidth "));
        assert!(kdl.contains(
            "\n            // The space between the edges of the surface and its contents.\n            padding {\n                // The measure in the horizontal axis.\n                x "
        ));
    }

    #[test]
    fn partial_config_overrides_only_the_given_values() {
        let config = parse(
            r##"
            width 400

            theme {
                button {
                    surface {
                        border-radius 0
                    }
                }

                notification {
                    title {
                        color "#ff000080"
                    }
                }
            }
            "##,
        )
        .unwrap();

        let defaults = Config::default();
        assert_eq!(config.width, 400.);
        assert_eq!(config.margin.x, defaults.margin.x);

        let button = &config.theme.button.surface;
        assert_eq!(button.border_radius, 0.);
        assert_eq!(button.padding.x, defaults.theme.button.surface.padding.x);

        let title = &config.theme.notification.title;
        assert_eq!(title.color.to_hex(), "#ff000080");
        assert_eq!(
            title.font.to_str(),
            defaults.theme.notification.title.font.to_str()
        );
    }

    #[test]
    fn invalid_configs_are_rejected_at_the_offending_node() {
        for (kdl, offending) in [
            // Invalid color.
            (
                r#"theme { bar { surface { background "nope"; }; }; }"#,
                r#""nope""#,
            ),
            // Invalid type.
            ("stack { max-count 2.5; }", "2.5"),
            // Unknown field.
            ("theme { bar { unknown 1; }; }", "unknown 1;"),
            // Block on a value.
            ("width 1 { x 1; }", "x 1;"),
            // Missing value.
            ("width { x 1; }", "width"),
            // Value on a block.
            ("margin 1", "1"),
            // Multiple values.
            ("width 1 2", "2"),
            // Duplicate field.
            ("width 1\nwidth 2", "width"),
        ] {
            let err = parse(kdl).expect_err(kdl);
            let label = err
                .related()
                .and_then(|mut related| related.next())
                .and_then(|err| err.labels())
                .and_then(|mut labels| labels.next())
                .expect(kdl);
            let span = label.offset()..label.offset() + label.len();
            assert_eq!(&kdl[span], offending, "{kdl}");
        }
    }

    #[test]
    fn syntax_errors_are_rejected() {
        assert!(parse("theme {").is_err());
    }

    #[test]
    fn written_fields_match_the_documented_fields() {
        fn check<T: Fields>(value: &T) {
            let fields = value.fields().into_iter().map(|(name, _)| name);
            assert!(fields.eq(T::FIELD_NAMES.iter().copied()));
        }

        let config = Config::default();
        let theme = &config.theme;
        check(&config);
        check(&config.margin);
        check(&config.stack);
        check(&config.spread);
        check(theme);
        check(&theme.notification);
        check(&theme.button);
        check(&theme.bar);
        check(&theme.notification.surface);
        check(&theme.notification.title);
    }
}
