use std::{
    env, fs,
    path::{Path, PathBuf},
};

use miette::IntoDiagnostic;

use crate::config::{Config, ConfigPatch, kdl::Setting};

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
    /// field as comments. The values that are the same as their defaults, and
    /// the ones that are unset, are commented out.
    pub fn to_kdl(&self) -> String {
        let mut out = String::new();
        self.write_fields(&Self::default(), &mut out, 0);
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
    fn configs_round_trip() {
        for kdl in [
            "",
            "width 400",
            r#"anchor { horizontal "center"; vertical "top"; }"#,
            "timeouts { default 5000; force-default true; }",
            r##"font "Inter 11"; colors { text "#000000"; }"##,
            r##"notification { title { font "Inter Italic 11"; }; }; bar { background "#ff0000"; }"##,
        ] {
            let config = parse(kdl).unwrap();
            assert_eq!(parse(&config.to_kdl()).unwrap(), config, "{kdl}");
        }
    }

    #[test]
    fn default_config_has_the_values_commented_out_under_their_docs() {
        let kdl = Config::default().to_kdl();
        assert!(kdl.starts_with("// The width of a notification card.\n// width 320.0\n"));
        assert!(kdl.contains(
            "\n    // The space between the edges and the contents.\n    padding {\n        // The measure in the horizontal axis.\n        // x "
        ));
        assert!(kdl.contains("\n    // Overrides the background color.\n    // background\n"));
    }

    #[test]
    fn changed_values_are_not_commented_out() {
        let kdl = parse("width 400\nmargin { x 0; }\nbar { color \"#ff0000\"; }")
            .unwrap()
            .to_kdl();
        assert!(kdl.contains("\nwidth 400.0\n"));
        assert!(kdl.contains("\n    x 0.0\n"));
        assert!(kdl.contains("\n    // y 32.0\n"));
        assert!(kdl.contains("\n    color \"#ff0000\"\n"));
    }

    #[test]
    fn partial_config_overrides_only_the_given_values() {
        let config = parse(
            r##"
            width 400

            actions {
                radius 0
            }

            notification {
                title {
                    color "#ff000080"
                }
            }
            "##,
        )
        .unwrap();

        let defaults = Config::default();
        assert_eq!(config.width, 400.);
        assert_eq!(config.anchor, defaults.anchor);
        assert_eq!(config.margin.x, defaults.margin.x);

        assert_eq!(config.actions.radius, 0.);
        assert_eq!(config.actions.padding, defaults.actions.padding);

        let title = &config.notification.title;
        assert_eq!(title.color.unwrap().to_hex(), "#ff000080");
        assert_eq!(title.font, defaults.notification.title.font);
    }

    #[test]
    fn invalid_configs_are_rejected_at_the_offending_node() {
        for (kdl, offending) in [
            // Invalid color.
            (r#"bar { background "nope"; }"#, r#""nope""#),
            // Invalid type.
            ("stack { max-count 2.5; }", "2.5"),
            // Invalid variant.
            (r#"anchor { vertical "middle"; }"#, r#""middle""#),
            // Unknown field.
            ("bar { unknown 1; }", "unknown 1;"),
            // Block on a value.
            ("width 1 { x 1; }", "x 1;"),
            // Missing value.
            ("width { x 1; }", "width"),
            // Missing optional value.
            ("bar { background; }", "background"),
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
        assert!(parse("notification {").is_err());
    }
}
