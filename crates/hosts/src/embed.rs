//! Routing of embedded code by its language tag: a `<script lang>`, a
//! markup_fmt extension hint, or a Markdown fence info string.

use fmtkit_core::Lang;
use oxc_formatter_css::CssVariant;

/// What an embedded block holds and which formatter takes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Embedded {
    Script(Lang),
    Style(CssVariant),
}

impl Embedded {
    /// Classify a language tag case-insensitively. Anything else is left as written.
    pub(crate) fn from_tag(tag: &str) -> Option<Self> {
        let embedded = match tag.to_ascii_lowercase().as_str() {
            "ts" | "typescript" => Self::Script(Lang::Ts),
            "tsx" => Self::Script(Lang::Tsx),
            "mts" => Self::Script(Lang::Mts),
            "cts" => Self::Script(Lang::Cts),
            "js" | "javascript" => Self::Script(Lang::Js),
            "jsx" => Self::Script(Lang::Jsx),
            "mjs" => Self::Script(Lang::Mjs),
            "cjs" => Self::Script(Lang::Cjs),
            "css" | "postcss" => Self::Style(CssVariant::Css),
            "scss" => Self::Style(CssVariant::Scss),
            "less" => Self::Style(CssVariant::Less),
            _ => return None,
        };

        Some(embedded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_tags_follow_v1_and_keep_the_dialect() {
        for tag in ["ts", "TS", "tsx", "js", "JSX", "typescript", "javascript", "mjs", "cjs", "mts", "cts"] {
            assert!(matches!(Embedded::from_tag(tag), Some(Embedded::Script(_))), "{tag}");
        }

        assert_eq!(Embedded::from_tag("tsx"), Some(Embedded::Script(Lang::Tsx)));
        assert_eq!(Embedded::from_tag("JSX"), Some(Embedded::Script(Lang::Jsx)));
        assert_eq!(Embedded::from_tag("typescript"), Some(Embedded::Script(Lang::Ts)));
        assert_eq!(Embedded::from_tag("javascript"), Some(Embedded::Script(Lang::Js)));
    }

    #[test]
    fn style_tags_cover_what_oxc_supports() {
        assert_eq!(Embedded::from_tag("css"), Some(Embedded::Style(CssVariant::Css)));
        assert_eq!(Embedded::from_tag("SCSS"), Some(Embedded::Style(CssVariant::Scss)));
        assert_eq!(Embedded::from_tag("less"), Some(Embedded::Style(CssVariant::Less)));
        assert_eq!(Embedded::from_tag("postcss"), Some(Embedded::Style(CssVariant::Css)));
    }

    #[test]
    fn other_tags_are_left_alone() {
        for tag in ["json", "bash", "sh", "yaml", "html", "sass", "stylus", "coffee", ""] {
            assert_eq!(Embedded::from_tag(tag), None, "{tag}");
        }
    }
}
