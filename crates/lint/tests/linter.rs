//! The oxc bridge: configuration, diagnostics, fixes, Vue sections and ignores.

use std::collections::BTreeMap;

use fmtkit_config::Lint;
use fmtkit_core::{Lang, Severity};
use fmtkit_lint::{LintError, Linter, SYNTAX_RULE};
use serde_json::{Value, json};

fn only(rules: &[(&str, Value)]) -> Lint {
    Lint {
        bundled: false,
        rules: rules.iter().map(|(name, value)| ((*name).to_owned(), value.clone())).collect::<BTreeMap<_, _>>(),
        ignore: Vec::new(),
        ..Default::default()
    }
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
fn bundled_policy_requires_a_reason_for_type_checker_directives() {
    let linter = Linter::new(&Lint::default()).unwrap();
    let flagged = |source: &str| rules_of(&linter, "a.ts", Lang::Ts, source).into_iter().filter(|(rule, ..)| rule == "typescript/ban-ts-comment").count();

    for bare in [
        "// @ts-ignore
let a = 1;
",
        "// @ts-expect-error
let a = 1;
",
        "// @ts-nocheck
let a = 1;
",
    ] {
        assert_eq!(flagged(bare), 1, "{bare}");
    }

    for reasoned in [
        "// @ts-ignore: the fixture is untyped
let a = 1;
",
        "// @ts-expect-error -- checks the runtime guard
let a = 1;
",
        "// @ts-nocheck generated file
let a = 1;
",
    ] {
        assert_eq!(flagged(reasoned), 0, "{reasoned}");
    }
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

fn reporting_unused(setting: &str, rules: &[(&str, Value)]) -> Linter {
    let mut rules = rules.to_vec();

    rules.push((fmtkit_lint::UNUSED_DIRECTIVE_RULE, json!(setting)));
    linter(&rules)
}

#[test]
fn unused_directives_are_reported_at_the_configured_severity() {
    let source = "// oxlint-disable-next-line eqeqeq\nif (a === 1) {}\n// oxlint-disable-next-line eqeqeq\nif (a == 2) {}\n";

    assert_eq!(rules_of(&linter(&[("eqeqeq", json!("error"))]), "a.js", Lang::Js, source), []);

    for (setting, severity) in [("warn", Severity::Warning), ("error", Severity::Error)] {
        let diagnostics = reporting_unused(setting, &[("eqeqeq", json!("error"))]).lint("a.js", Lang::Js, source, false).diagnostics;
        let found: Vec<_> = diagnostics.iter().map(|d| (d.rule.as_str(), d.line, d.column, d.severity)).collect();

        assert_eq!(found, [(fmtkit_lint::UNUSED_DIRECTIVE_RULE, 1, 1, severity)], "{setting}");
    }

    assert_eq!(rules_of(&reporting_unused("off", &[("eqeqeq", json!("error"))]), "a.js", Lang::Js, source), []);
}

#[test]
fn a_directive_that_suppresses_a_native_finding_is_used() {
    let linter = reporting_unused("error", &[("no-only-tests/no-only-tests", json!("error"))]);
    let source = "// oxlint-disable-next-line no-only-tests/no-only-tests\ndescribe.only('a', () => {});\n";

    assert_eq!(rules_of(&linter, "a.test.ts", Lang::Ts, source), []);
    assert_eq!(
        rules_of(&linter, "a.test.ts", Lang::Ts, "// oxlint-disable-next-line no-only-tests/no-only-tests\ndescribe('a', () => {});\n"),
        [(fmtkit_lint::UNUSED_DIRECTIVE_RULE.to_owned(), 1, 1)]
    );
}

#[test]
fn fix_removes_an_unused_oxlint_directive_only() {
    let linter = reporting_unused("error", &[("eqeqeq", json!("error"))]);
    let oxlint = linter.lint("a.js", Lang::Js, "// oxlint-disable-next-line eqeqeq\nif (a === 1) {}\n", true);

    assert_eq!(oxlint.fixed.as_deref(), Some("if (a === 1) {}\n"));
    assert_eq!(oxlint.diagnostics, []);

    // ESLint may still need an `eslint-` directive, so removing one is only a suggestion.
    let eslint = linter.lint("a.js", Lang::Js, "// eslint-disable-next-line eqeqeq\nif (a === 1) {}\n", true);

    assert_eq!(eslint.fixed, None);
    assert_eq!(eslint.diagnostics.len(), 1);
}

#[test]
fn directives_for_type_aware_rules_are_never_unused() {
    let linter = reporting_unused("error", &[]);

    for name in ["typescript/no-floating-promises", "@typescript-eslint/no-floating-promises", "@typescript-eslint/no-unsafe-assignment"] {
        let source = format!("// oxlint-disable-next-line {name}\nf();\n");

        assert_eq!(rules_of(&linter, "a.ts", Lang::Ts, &source), [], "{name}");
    }

    assert_eq!(
        rules_of(&linter, "a.ts", Lang::Ts, "// oxlint-disable-next-line typescript/no-explicit-any\nf();\n"),
        [(fmtkit_lint::UNUSED_DIRECTIVE_RULE.to_owned(), 1, 1)]
    );

    // Rule names end where oxc_linter ends them: at `--`, or at a `-` with
    // whitespace on both sides, and they split on any whitespace.
    for directive in [
        "// oxlint-disable-next-line typescript/no-floating-promises - awaited by the caller",
        "// oxlint-disable-next-line typescript/no-floating-promises -- awaited - by the caller",
        "// oxlint-disable-next-line typescript/no-floating-promises\u{a0}@typescript-eslint/no-unsafe-assignment",
    ] {
        assert_eq!(rules_of(&linter, "a.ts", Lang::Ts, &format!("{directive}\nf();\n")), [], "{directive}");
    }

    // With a rule fmtkit runs beside it, the directive is reported but kept.
    let mixed = "/* oxlint-disable-next-line typescript/no-floating-promises, eqeqeq -- awaited by the caller */\nf();\n";
    let linted = linter.lint("a.ts", Lang::Ts, mixed, true);

    assert_eq!(linted.fixed, None);
    assert_eq!(linted.diagnostics.len(), 1);
}

#[test]
fn unused_directives_in_vue_scripts_are_reported() {
    let linter = reporting_unused("error", &[("eqeqeq", json!("error"))]);
    let source = "<script>\n// oxlint-disable-next-line eqeqeq\nif (a === 1) {}\n</script>\n";

    assert_eq!(rules_of(&linter, "a.vue", Lang::Vue, source), [(fmtkit_lint::UNUSED_DIRECTIVE_RULE.to_owned(), 2, 1)]);
}

#[test]
fn the_bundled_policy_reports_unused_directives_unless_turned_off() {
    let source = "// oxlint-disable-next-line eqeqeq\nexport const a = 1;\n";

    assert_eq!(rules_of(&Linter::new(&Lint::default()).unwrap(), "a.ts", Lang::Ts, source), [(fmtkit_lint::UNUSED_DIRECTIVE_RULE.to_owned(), 1, 1)]);
    let off = Lint { rules: [(fmtkit_lint::UNUSED_DIRECTIVE_RULE.to_owned(), json!("off"))].into(), ..Lint::default() };

    assert_eq!(rules_of(&Linter::new(&off).unwrap(), "a.ts", Lang::Ts, source), []);

    for setting in [json!("loud"), json!(["error", {}])] {
        let error = Linter::new(&only(&[(fmtkit_lint::UNUSED_DIRECTIVE_RULE, setting.clone())])).err().expect("an invalid setting is refused");

        assert!(matches!(&error, LintError::Rule { rule, .. } if rule == fmtkit_lint::UNUSED_DIRECTIVE_RULE), "{setting}: {error}");
    }
}

#[test]
fn unused_directives_can_be_exempted() {
    let exempt = vec![fmtkit_config::Exemption { rule: fmtkit_lint::UNUSED_DIRECTIVE_RULE.into(), reason: "generated".into(), paths: vec!["gen/".into()] }];
    let config = Lint { exempt, ..only(&[("eqeqeq", json!("error")), (fmtkit_lint::UNUSED_DIRECTIVE_RULE, json!("error"))]) };
    let linter = Linter::new(&config).unwrap();
    let source = "// oxlint-disable-next-line eqeqeq\nexport const a = 1;\n";

    assert_eq!(rules_of(&linter, "gen/a.ts", Lang::Ts, source), []);
    assert_eq!(rules_of(&linter, "src/a.ts", Lang::Ts, source), [(fmtkit_lint::UNUSED_DIRECTIVE_RULE.to_owned(), 1, 1)]);
}

#[test]
fn type_aware_rules_are_config_errors() {
    for name in ["typescript/no-floating-promises", "@typescript-eslint/await-thenable"] {
        let error = Linter::new(&only(&[(name, json!("error"))])).err().unwrap_or_else(|| panic!("{name} should be rejected"));

        assert!(matches!(&error, LintError::Rule { rule, message } if rule == name && message.contains("type-aware")), "{name}: {error}");
        assert!(Linter::new(&only(&[(name, json!("off"))])).is_ok(), "{name} may be turned off");
    }
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

fn exempt(rule: &str, paths: &[&str]) -> fmtkit_config::Exemption {
    fmtkit_config::Exemption { rule: rule.into(), reason: "test".into(), paths: paths.iter().map(|p| (*p).to_owned()).collect() }
}

#[test]
fn exemptions_drop_one_oxc_rule_in_matching_files_only() {
    let config = Lint {
        exempt: vec![exempt("eslint/require-await", &["**/*.test.ts", "src/keep.ts"])],
        ..only(&[("require-await", json!("error")), ("eqeqeq", json!("error"))])
    };
    let linter = Linter::new(&config).unwrap();
    let source = "export async function a() { return 1; }\nif (a == 1) {}\n";
    let rules = |rel: &str| rules_of(&linter, rel, Lang::Ts, source).into_iter().map(|(rule, ..)| rule).collect::<Vec<_>>();

    assert_eq!(rules("src/a.ts"), ["eslint/require-await", "eslint/eqeqeq"]);
    assert_eq!(rules("src/deep/a.test.ts"), ["eslint/eqeqeq"]);
    assert_eq!(rules("src/keep.ts"), ["eslint/eqeqeq"]);
    assert_eq!(rules("other/src/keep.ts"), ["eslint/require-await", "eslint/eqeqeq"]);
}

#[test]
fn exemptions_drop_the_fixes_of_the_rule_they_exempt() {
    let config = Lint { exempt: vec![exempt("prefer-const", &["a.ts"])], ..only(&[("prefer-const", json!("error"))]) };
    let linter = Linter::new(&config).unwrap();
    let source = "let a = 1;\nexport { a };\n";

    assert_eq!(linter.lint("a.ts", Lang::Ts, source, true), fmtkit_lint::Linted::default());
    assert!(linter.lint("b.ts", Lang::Ts, source, true).fixed.is_some());
}

#[test]
fn exemptions_drop_native_rules() {
    let rule = "no-only-tests/no-only-tests";
    let config = Lint { exempt: vec![exempt(rule, &["e2e/"])], ..only(&[(rule, json!("error"))]) };
    let linter = Linter::new(&config).unwrap();
    let source = "describe.only('a', () => {});\n";

    assert_eq!(rules_of(&linter, "src/a.test.ts", Lang::Ts, source).len(), 1);
    assert_eq!(rules_of(&linter, "e2e/flows/a.test.ts", Lang::Ts, source), []);
}

#[test]
fn exemptions_accept_rules_the_bundled_policy_enables() {
    let config = Lint { exempt: vec![exempt("@typescript-eslint/no-explicit-any", &["a.ts"]), exempt("eslint/require-await", &["a.ts"])], ..Lint::default() };

    assert!(Linter::new(&config).is_ok());
}

#[test]
fn exemptions_of_unknown_or_disabled_rules_are_config_errors() {
    let refused = |rule: &str, rules: &[(&str, Value)]| {
        let config = Lint { exempt: vec![exempt(rule, &["a.ts"])], ..only(rules) };

        match Linter::new(&config) {
            Err(LintError::Exempt { rule: named, message }) if named == rule => message,
            other => panic!("{rule}: {:?}", other.err().map(|e| e.to_string())),
        }
    };

    assert_eq!(refused("eslint/not-a-rule", &[]), "unknown rule");
    assert_eq!(refused("some-plugin/rule", &[]), "unknown rule");
    assert!(refused("eqeqeq", &[]).contains("off"));
    assert!(refused("eqeqeq", &[("eqeqeq", json!("off"))]).contains("off"));
    assert!(refused("no-only-tests/no-only-tests", &[]).contains("off"));
    assert!(refused("eqeqeq", &[("curly", json!("error"))]).contains("off"));

    let config = Lint { exempt: vec![exempt("eqeqeq", &["a{b,c"])], ..only(&[("eqeqeq", json!("error"))]) };

    assert!(matches!(Linter::new(&config), Err(LintError::Exempt { .. })));
}
