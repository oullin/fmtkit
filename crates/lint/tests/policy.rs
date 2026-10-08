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

    assert_eq!(snapshot["version"], "2.0.1");
    assert!(snapshot["source"].as_str().unwrap().ends_with("cb48b60893ebf3d6ec0655b295fce70fb51c376a/index.js"));
}

#[test]
fn rules_match_upstream_except_existing_settings() {
    let (_, upstream, shipped) = load();
    let existing = json!({
        "eqeqeq": ["error", "always"],
        "no-unused-vars": ["error", { "args": "after-used", "argsIgnorePattern": "^_", "caughtErrors": "none", "varsIgnorePattern": "^_" }],
        "unicorn/catch-error-name": ["error", { "name": "cause" }],
    });

    for (name, value) in upstream["rules"].as_object().unwrap() {
        let normalized = normalize(name);
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

#[test]
fn overrides_are_the_generic_typescript_override() {
    let (_, upstream, shipped) = load();
    let mut expected = upstream["overrides"][0].clone();

    expected["rules"] = normalize_rules(&expected["rules"]);

    assert_eq!(shipped["overrides"], json!([expected]));
}
