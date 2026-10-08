//! The oxc bridge: configuration, diagnostics, fixes, Vue sections and ignores.

use std::collections::BTreeMap;

use fmtkit_config::Lint;
use fmtkit_core::{Lang, Severity};
use fmtkit_lint::{LintError, Linter, SYNTAX_RULE};
use serde_json::{Value, json};

fn only(rules: &[(&str, Value)]) -> Lint {
    Lint { bundled: false, rules: rules.iter().map(|(name, value)| ((*name).to_owned(), value.clone())).collect::<BTreeMap<_, _>>(), ignore: Vec::new() }
}

fn linter(rules: &[(&str, Value)]) -> Linter {
    Linter::new(&only(rules)).expect("config builds")
}

fn rules_of(linter: &Linter, rel: &str, lang: Lang, source: &str) -> Vec<(String, u32, u32)> {
    linter.lint(rel, lang, source, false).diagnostics.into_iter().map(|d| (d.rule, d.line, d.column)).collect()
}

#[test]
fn bundled_policy_builds() {
    let linter = Linter::new(&Lint::default()).expect("bundled policy builds");
    let linted = linter.lint("src/a.ts", Lang::Ts, "export const a = 1;\n", false);

    assert_eq!(linted, fmtkit_lint::Linted::default());
}

#[test]
fn unknown_rules_are_config_errors() {
    for name in ["eslint/not-a-rule", "not-a-rule", "unicorn/not-a-rule", "some-plugin/rule", "@scope/rule", "perfectionist/not-a-rule"] {
        let error = Linter::new(&only(&[(name, json!("error"))])).err().unwrap_or_else(|| panic!("{name} should be rejected"));

        assert!(matches!(&error, LintError::Rule { rule, .. } if rule == name), "{name}: {error}");
    }
}

#[test]
fn invalid_settings_are_config_errors() {
    for setting in [json!("loud"), json!([]), json!(3), json!({ "level": "error" })] {
        assert!(Linter::new(&only(&[("eqeqeq", setting.clone())])).is_err(), "{setting}");
    }
}

#[test]
fn invalid_ignore_patterns_are_config_errors() {
    let config = Lint { ignore: vec!["a{b,c".into()], ..only(&[]) };

    assert!(matches!(Linter::new(&config), Err(LintError::Ignore { .. })));
}

#[test]
fn diagnostics_use_oxlint_names_and_positions() {
    let linter = linter(&[("eqeqeq", json!("error")), ("@typescript-eslint/no-explicit-any", json!("warn"))]);
    let linted = linter.lint("src/a.ts", Lang::Ts, "let a: any = 1;\nif (a == 2) {}\n", false);
    let found: Vec<_> = linted.diagnostics.iter().map(|d| (d.rule.as_str(), d.line, d.column, d.severity)).collect();

    assert_eq!(found, [("typescript/no-explicit-any", 1, 8, Severity::Warning), ("eslint/eqeqeq", 2, 7, Severity::Error)]);
    assert!(linted.diagnostics.iter().all(|d| d.file == "src/a.ts" && !d.message.is_empty()));
}

#[test]
fn overrides_replace_aliased_policy_entries() {
    let off = Linter::new(&Lint { rules: BTreeMap::from([("eslint/eqeqeq".into(), json!("off"))]), ..Lint::default() }).unwrap();

    assert!(rules_of(&off, "a.ts", Lang::Ts, "if (1 == 2) {}\n").iter().all(|(rule, ..)| rule != "eslint/eqeqeq"));

    let on = Linter::new(&Lint::default()).unwrap();

    assert!(rules_of(&on, "a.ts", Lang::Ts, "if (1 == 2) {}\n").iter().any(|(rule, ..)| rule == "eslint/eqeqeq"));
}

#[test]
fn plugins_are_enabled_by_rule_names() {
    let linter = linter(&[("unicorn/prefer-node-protocol", json!("error"))]);

    assert_eq!(rules_of(&linter, "a.mjs", Lang::Mjs, "import fs from 'fs';\nfs;\n"), [("unicorn/prefer-node-protocol".to_owned(), 1, 16)]);
}

#[test]
fn safe_fixes_are_applied_and_relinted() {
    let linter = linter(&[("unicorn/prefer-node-protocol", json!("error"))]);
    let linted = linter.lint("a.mjs", Lang::Mjs, "import fs from 'fs';\nimport path from 'path';\nfs, path;\n", true);

    assert_eq!(linted.fixed.as_deref(), Some("import fs from 'node:fs';\nimport path from 'node:path';\nfs, path;\n"));
    assert_eq!(linted.diagnostics, []);
}

#[test]
fn nothing_to_fix_leaves_fixed_empty() {
    let linter = linter(&[("eqeqeq", json!("error"))]);
    let linted = linter.lint("a.js", Lang::Js, "if (a === 1) {}\n", true);

    assert_eq!(linted, fmtkit_lint::Linted::default());
}

#[test]
fn syntax_errors_are_reported_and_stop_linting() {
    let linter = linter(&[("eqeqeq", json!("error"))]);
    let linted = linter.lint("a.js", Lang::Js, "if (a == 1) {\n", true);

    assert!(linted.fixed.is_none());
    assert_ne!(linted.diagnostics.len(), 0);
    assert!(linted.diagnostics.iter().all(|d| d.rule == SYNTAX_RULE && d.severity == Severity::Error));
}

#[test]
fn vue_script_blocks_map_to_file_positions() {
    let linter = linter(&[("eqeqeq", json!("error")), ("unicorn/prefer-node-protocol", json!("error"))]);
    let source = "<template>\n  <div />\n</template>\n\n<script setup lang=\"ts\">\nimport fs from 'fs';\nif (fs == 1) {}\n</script>\n";

    assert_eq!(rules_of(&linter, "a.vue", Lang::Vue, source), [("unicorn/prefer-node-protocol".to_owned(), 6, 16), ("eslint/eqeqeq".to_owned(), 7, 8)]);

    let linted = linter.lint("a.vue", Lang::Vue, source, true);

    assert_eq!(linted.fixed.as_deref(), Some(source.replace("'fs'", "'node:fs'").as_str()));
}

#[test]
fn vue_without_script_has_nothing_to_lint() {
    let linter = linter(&[("eqeqeq", json!("error"))]);

    assert_eq!(linter.lint("a.vue", Lang::Vue, "<template><div /></template>\n", true), fmtkit_lint::Linted::default());
}

#[test]
fn disable_directives_apply() {
    let linter = linter(&[("eqeqeq", json!("error"))]);
    let source = "// eslint-disable-next-line eqeqeq\nif (a == 1) {}\nif (a == 2) {}\n";

    assert_eq!(rules_of(&linter, "a.js", Lang::Js, source), [("eslint/eqeqeq".to_owned(), 3, 7)]);
}

#[test]
fn ignore_patterns_match_paths_and_parents() {
    let config = Lint { ignore: vec!["dist/".into(), "*.gen.ts".into(), "/root-only.ts".into()], ..only(&[]) };
    let linter = Linter::new(&config).unwrap();

    assert!(linter.ignores("dist/a.ts"));
    assert!(linter.ignores("packages/web/dist/a.ts"));
    assert!(linter.ignores("src/schema.gen.ts"));
    assert!(linter.ignores("root-only.ts"));
    assert!(!linter.ignores("src/root-only.ts"));
    assert!(!linter.ignores("src/a.ts"));
    assert!(!Linter::new(&only(&[])).unwrap().ignores("dist/a.ts"));
}

#[test]
fn linter_is_shareable_across_threads() {
    let linter = linter(&[("eqeqeq", json!("error"))]);

    std::thread::scope(|scope| {
        for _ in 0..4 {
            scope.spawn(|| assert_eq!(rules_of(&linter, "a.js", Lang::Js, "if (a == 1) {}\n").len(), 1));
        }
    });
}
