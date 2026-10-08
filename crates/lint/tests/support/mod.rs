//! Running one rule the way ESLint's RuleTester does.

#![allow(dead_code)]

use fmtkit_config::Lint;
use fmtkit_core::Lang;
use fmtkit_lint::{Linted, Linter};
use serde_json::{Value, json};

/// A linter with nothing but `rule`, configured as `["error", ...options]`.
pub fn only(rule: &str, options: &[Value]) -> Linter {
    let mut setting = vec![json!("error")];

    setting.extend(options.iter().cloned());

    let config = Lint { bundled: false, rules: [(rule.to_owned(), Value::Array(setting))].into(), ignore: Vec::new() };

    Linter::new(&config).unwrap_or_else(|e| panic!("{rule}: {e}"))
}

/// The messages `linter` reports for `code`, linted as `file`.
pub fn messages(linter: &Linter, file: &str, code: &str) -> Vec<String> {
    lint(linter, file, code, false).diagnostics.into_iter().map(|d| d.message).collect()
}

pub fn lint(linter: &Linter, file: &str, code: &str, fix: bool) -> Linted {
    let lang = Lang::from_path(std::path::Path::new(file)).expect("a lintable file name");

    linter.lint(file, lang, code, fix)
}

/// The fixed source, or `code` itself when nothing was fixed.
pub fn output(linter: &Linter, file: &str, code: &str) -> String {
    lint(linter, file, code, true).fixed.unwrap_or_else(|| code.to_owned())
}
