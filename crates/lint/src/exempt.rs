//! `[[lint.exempt]]`: rules that do not apply to some files. A finding of an
//! exempted rule in a matching file is dropped, together with its fix; every
//! other rule still applies to that file.

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use oxc_linter::normalize_rule_name;

/// The exemptions of one linter, keyed the way rules are merged.
#[derive(Default)]
pub struct Exempt {
    entries: Vec<(String, Gitignore)>,
}

/// The rule keys exempted for one file.
pub struct ForFile<'e>(Vec<&'e str>);

impl Exempt {
    pub fn push(&mut self, key: String, matcher: Gitignore) {
        self.entries.push((key, matcher));
    }

    /// The rules exempted for a repository-relative path.
    pub fn for_file(&self, rel: &str) -> ForFile<'_> {
        ForFile(self.entries.iter().filter(|(_, matcher)| matched(matcher, rel)).map(|(key, _)| key.as_str()).collect())
    }
}

impl ForFile<'_> {
    /// Whether a native rule, named as it registers itself, is exempted.
    pub fn native(&self, rule: &str) -> bool {
        self.0.contains(&rule)
    }

    /// Whether an oxc_linter rule, named `plugin/rule` as its diagnostics are,
    /// is exempted.
    pub fn oxc(&self, code: &str) -> bool {
        !self.0.is_empty() && self.0.contains(&normalize_rule_name(code).as_str())
    }
}

/// gitignore semantics, as `[lint] ignore` matches.
pub fn matcher(patterns: &[String]) -> Result<Gitignore, (String, String)> {
    let mut builder = GitignoreBuilder::new("");

    for pattern in patterns {
        builder.add_line(None, pattern).map_err(|e| (pattern.clone(), e.to_string()))?;
    }

    builder.build().map_err(|e| (patterns.join(", "), e.to_string()))
}

pub fn matched(matcher: &Gitignore, rel: &str) -> bool {
    matcher.matched_path_or_any_parents(rel, false).is_ignore()
}
