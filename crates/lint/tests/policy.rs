//! The bundled policy against the pinned `@nkzw/oxlint-config` snapshot it was
//! derived from. Replaces v1's `scripts/verify-oxlint-parity.ts`.

use serde_json::{Map, Value, json};

const SNAPSHOT: &str = include_str!("../policy/nkzw-policy.snapshot.json");

fn normalize(name: &str) -> String {
    if let Some(rest) = name.strip_prefix("@typescript-eslint/") {
        return format!("typescript/{rest}");
    }

    if let Some(rest) = name.strip_prefix("import-x/") {
        return format!("import/{rest}");
    }

    name.to_owned()
}

/// Whether oxc_linter runs this normalized rule only with type information,
/// which fmtkit does not have; the bundled policy leaves these rules out.
fn type_aware(name: &str) -> bool {
    let Some(rule) = name.strip_prefix("typescript/") else { return false };

    oxc_linter::rules::RULES.iter().any(|r| r.plugin_name() == "typescript" && r.name() == rule && r.is_tsgolint_rule())
}

fn normalize_rules(rules: &Value) -> Value {
    let rules = rules.as_object().expect("rules is an object");

    Value::Object(rules.iter().map(|(name, value)| (normalize(name), value.clone())).collect())
}

fn load() -> (Map<String, Value>, Value, Map<String, Value>) {
    let snapshot: Map<String, Value> = serde_json::from_str(SNAPSHOT).expect("snapshot is JSON");
    let shipped: Map<String, Value> = serde_json::from_str(fmtkit_lint::POLICY).expect("policy is JSON");
    let upstream = snapshot["config"].clone();

    (snapshot, upstream, shipped)
}

#[test]
fn snapshot_is_the_pinned_release() {
    let (snapshot, ..) = load();

    assert_eq!(snapshot["version"], "3.0.0");
    assert!(snapshot["source"].as_str().unwrap().ends_with("fca2f5b6446c3df7a8ce44688c2847898988b593/with-sorting.js"));
}

/// fmtkit ships the `with-sorting` preset, so the six `perfectionist/*`
/// sorting rules stay on.
#[test]
fn rules_match_upstream_except_existing_settings() {
    let (_, upstream, shipped) = load();
    let existing = json!({
        "eqeqeq": ["error", "always"],
        "no-unused-vars": ["error", { "args": "after-used", "argsIgnorePattern": "^_", "caughtErrors": "none", "varsIgnorePattern": "^_" }],
        "typescript/ban-ts-comment": ["error", { "ts-expect-error": "allow-with-description", "ts-ignore": "allow-with-description", "ts-nocheck": "allow-with-description" }],
        "unicorn/catch-error-name": ["error", { "name": "cause" }],
    });

    for (name, value) in upstream["rules"].as_object().unwrap() {
        let normalized = normalize(name);

        if type_aware(&normalized) {
            assert!(shipped["rules"].get(&normalized).is_none(), "type-aware rule {normalized} cannot run without type information");
            continue;
        }

        let expected = existing.get(&normalized).unwrap_or(value);

        assert_eq!(shipped["rules"].get(&normalized), Some(expected), "bundled rule {normalized} drifted from the pinned policy");
    }
}

#[test]
fn categories_env_and_plugins_match() {
    let (_, upstream, shipped) = load();

    assert_eq!(shipped["categories"]["correctness"], "error");
    assert_eq!(shipped["env"], upstream["env"]);

    for plugin in upstream["plugins"].as_array().unwrap() {
        assert!(shipped["plugins"].as_array().unwrap().contains(plugin), "missing native plugin {plugin}");
    }
}

/// v1 loaded the JS plugins through oxlint's `jsPlugins`; 2.0 runs them as
/// native rules, so every upstream rule of a JS plugin must be in the registry.
#[test]
fn js_plugin_rules_are_native() {
    let (_, upstream, shipped) = load();
    let native: Vec<&str> = fmtkit_lint::rules::registry().map(|(name, _)| name).collect();

    assert!(!shipped.contains_key("jsPlugins"));

    for plugin in upstream["jsPlugins"].as_array().unwrap() {
        let prefix = format!("{}/", plugin["name"].as_str().unwrap());

        for name in upstream["rules"].as_object().unwrap().keys().filter(|name| name.starts_with(&prefix)) {
            assert!(native.contains(&name.as_str()), "{name} has no native implementation");
        }
    }
}

/// The TypeScript and test-file overrides; upstream's own `server/` override
/// is specific to its repository.
#[test]
fn overrides_are_the_typescript_and_test_overrides() {
    let (_, upstream, shipped) = load();
    let mut expected: Vec<Value> = upstream["overrides"].as_array().unwrap()[..2].to_vec();

    for entry in &mut expected {
        let rules = normalize_rules(&entry["rules"]);

        entry["rules"] = Value::Object(rules.as_object().unwrap().iter().filter(|(name, _)| !type_aware(name)).map(|(n, v)| (n.clone(), v.clone())).collect());
    }

    assert_eq!(shipped["overrides"], Value::Array(expected));
}

/// The type-aware rules the snapshot enables, which fmtkit leaves out: every
/// one of them, and only them, is a tsgolint rule in oxc_linter.
#[test]
fn only_type_aware_rules_are_left_out() {
    let (_, upstream, shipped) = load();
    let mut left_out: Vec<String> =
        upstream["rules"].as_object().unwrap().keys().map(|name| normalize(name)).filter(|name| shipped["rules"].get(name).is_none()).collect();

    left_out.extend(
        upstream["overrides"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|o| o["rules"].as_object().unwrap().keys())
            .map(|name| normalize(name))
            .filter(|name| type_aware(name)),
    );
    left_out.sort();

    assert_eq!(
        left_out,
        [
            "typescript/await-thenable",
            "typescript/no-array-delete",
            "typescript/no-duplicate-type-constituents",
            "typescript/no-floating-promises",
            "typescript/no-for-in-array",
            "typescript/no-misused-promises",
            "typescript/no-misused-spread",
            "typescript/no-unsafe-unary-minus",
            "typescript/require-array-sort-compare",
            "typescript/return-await",
        ]
    );
}

#[test]
fn unused_directives_are_errors_without_type_information() {
    let (_, upstream, shipped) = load();

    assert_eq!(upstream["options"]["reportUnusedDisableDirectives"], "error");
    assert_eq!(shipped["rules"][fmtkit_lint::UNUSED_DIRECTIVE_RULE], "error");
    assert!(shipped.get("options").is_none(), "fmtkit configures options as rules");
}

/// Catch observers are project-specific, so upgrading the bundled policy must
/// leave the catch rules opt-in.
#[test]
fn observability_rules_are_opt_in() {
    let (_, _, shipped) = load();

    for (name, _) in fmtkit_lint::rules::observability::RULES {
        assert_eq!(shipped["rules"][name], "off", "{name} must remain opt-in");
    }

    let linter = fmtkit_lint::Linter::new(&fmtkit_config::Lint::default()).unwrap();
    let result = linter.lint("a.ts", fmtkit_core::Lang::Ts, "try {} catch {}\nrequest.catch(() => {});", false);

    assert!(result.diagnostics.iter().all(|diagnostic| !diagnostic.rule.starts_with("observability/")));
}

#[test]
fn boundaries_rules_are_opt_in() {
    let (_, _, shipped) = load();

    for (name, _) in fmtkit_lint::rules::boundaries::RULES {
        assert_eq!(shipped["rules"][name], "off", "{name} must remain opt-in");
    }

    let linter = fmtkit_lint::Linter::new(&fmtkit_config::Lint::default()).unwrap();
    let source = "export class Lifecycle {}\nexport class Other {}\nimport x from './x';\ntype A = any | null;\ntype B = z.infer<typeof x>;\nReflect.get(x, 'key');\ntypeof x;";
    let result = linter.lint("src/index.ts", fmtkit_core::Lang::Ts, source, false);

    assert!(result.diagnostics.iter().all(|diagnostic| !diagnostic.rule.starts_with("boundaries/")));
}

/// The `perf/*` rules suit codebases that chose them, so the bundled policy
/// leaves them out and a default run reports none of them.
#[test]
fn perf_rules_are_opt_in() {
    let (_, _, shipped) = load();

    for (name, _) in fmtkit_lint::rules::perf::RULES {
        assert!(shipped["rules"].get(name).is_none_or(|setting| setting == "off"), "{name} must remain opt-in");
    }

    let linter = fmtkit_lint::Linter::new(&fmtkit_config::Lint::default()).unwrap();
    let source = "export async function f(ids: string[], s: string) {\n\tfor (const id of ids) {\n\t\tawait db.select().from(t);\n\t}\n\treturn [/a/u.test(s), new URL(s).href, new URL(s).origin];\n}\n";
    let result = linter.lint("src/index.ts", fmtkit_core::Lang::Ts, source, false);

    assert!(result.diagnostics.iter().all(|diagnostic| !diagnostic.rule.starts_with("perf/")));
}

/// The `naming/*` rules need a repository's workspace layout or a naming
/// budget it chose, so the bundled policy leaves them out.
#[test]
fn naming_rules_are_opt_in() {
    let (_, _, shipped) = load();

    for (name, _) in fmtkit_lint::rules::naming::RULES {
        assert!(shipped["rules"].get(name).is_none_or(|setting| setting == "off"), "{name} must remain opt-in");
    }

    let linter = fmtkit_lint::Linter::new(&fmtkit_config::Lint::default()).unwrap();
    let result = linter.lint("packages/twilio/src/twilio-client.ts", fmtkit_core::Lang::Ts, "export class TwilioClientFactory {}\n", false);

    assert!(result.diagnostics.iter().all(|diagnostic| !diagnostic.rule.starts_with("naming/")));
}

/// nkzw 3.0's `quality/*` rules are on. fmtkit shipped the same twelve rules
/// as `anti-slop/*`, and those names are now aliases: they configure, exempt
/// and suppress the `quality/*` rule.
#[test]
fn quality_rules_are_on_and_answer_to_their_anti_slop_names() {
    let (_, _, shipped) = load();
    let source = "export function f(value: object) {\n\treturn value;\n}\n";
    let with = |rules: &[(&str, &str)]| fmtkit_config::Lint {
        rules: rules.iter().map(|(name, setting)| ((*name).to_owned(), json!(setting))).collect(),
        ..fmtkit_config::Lint::default()
    };
    let rules_of = |config: &fmtkit_config::Lint, source: &str| -> Vec<(String, fmtkit_core::Severity)> {
        let linter = fmtkit_lint::Linter::new(config).unwrap();

        linter.lint("src/a.ts", fmtkit_core::Lang::Ts, source, false).diagnostics.into_iter().map(|d| (d.rule, d.severity)).collect()
    };
    let native: Vec<&str> = fmtkit_lint::rules::registry().map(|(name, _)| name).collect();

    for (name, _) in fmtkit_lint::rules::quality::RULES {
        let alias = fmtkit_lint::rules::quality::alias(name).unwrap();

        assert_eq!(shipped["rules"][name], "error", "{name} must be on");
        assert!(shipped["rules"].get(&alias).is_none(), "{alias} is an alias");
        assert!(!native.contains(&alias.as_str()), "{alias} is registered twice");
        assert_eq!(fmtkit_lint::rules::quality::alias_of(&alias), Some(*name));
    }

    let error = fmtkit_core::Severity::Error;

    assert_eq!(rules_of(&fmtkit_config::Lint::default(), source), [("quality/no-object-parameters".to_owned(), error)]);
    assert_eq!(
        rules_of(&with(&[("anti-slop/no-object-parameters", "warn")]), source),
        [("quality/no-object-parameters".to_owned(), fmtkit_core::Severity::Warning)]
    );
    assert_eq!(rules_of(&with(&[("anti-slop/no-object-parameters", "off")]), source), []);

    let suppressed = format!("// oxlint-disable-next-line anti-slop/no-object-parameters -- legacy\n{source}");

    assert_eq!(rules_of(&fmtkit_config::Lint::default(), &suppressed), []);

    let both = fmtkit_lint::Linter::new(&with(&[("anti-slop/no-object-parameters", "error"), ("quality/no-object-parameters", "off")]));

    assert!(matches!(both, Err(fmtkit_lint::LintError::Rule { rule, .. }) if rule == "anti-slop/no-object-parameters"));

    let exempt = vec![fmtkit_config::Exemption { rule: "anti-slop/no-object-parameters".into(), reason: "legacy".into(), paths: vec!["src/".into()] }];

    assert_eq!(rules_of(&fmtkit_config::Lint { exempt, ..fmtkit_config::Lint::default() }, source), []);
}
