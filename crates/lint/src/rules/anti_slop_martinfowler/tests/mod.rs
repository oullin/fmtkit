//! Table tests for the `anti-slop-martinfowler/*` rules, run through the anti-slop
//! [`check`](super::super::anti_slop::tests::check) harness.

mod global_data;
mod long_function;
mod long_parameter_list;
mod loops;
mod message_chains;
mod mutable_data;
mod mysterious_name;
mod repeated_switches;

use super::RULES;

#[test]
fn every_rule_is_registered() {
    let names: Vec<&str> = RULES.iter().map(|(name, _)| *name).collect();

    assert_eq!(
        names,
        [
            "anti-slop-martinfowler/global-data",
            "anti-slop-martinfowler/long-function",
            "anti-slop-martinfowler/long-parameter-list",
            "anti-slop-martinfowler/loops",
            "anti-slop-martinfowler/message-chains",
            "anti-slop-martinfowler/mutable-data",
            "anti-slop-martinfowler/mysterious-name",
            "anti-slop-martinfowler/repeated-switches",
        ]
    );
}

/// Hand-written: options follow upstream's schemas, which allow one object
/// with only the listed keys, typed, and integers at or above their minimum.
#[test]
fn options_follow_upstream_schemas() {
    let build = |rule: &str, options: &str| {
        let options: Vec<serde_json::Value> = serde_json::from_str(options).expect("options are a JSON array");
        let factory = RULES.iter().find(|(name, _)| *name == rule).map(|(_, factory)| *factory).expect("registered");

        factory(&options).map(|_| ())
    };

    for (name, _) in RULES {
        assert!(build(name, "[]").is_ok(), "{name} takes no options");
        assert!(build(name, "[{}]").is_ok(), "{name} takes an empty object");
        assert!(build(name, r#"[{"ignoreTests": false}]"#).is_ok(), "{name} takes ignoreTests");
        assert!(build(name, r#"[{"ignoreTests": 0}]"#).is_err(), "{name} rejects a non-boolean ignoreTests");
        assert!(build(name, r#"[{"unknown": true}]"#).is_err(), "{name} rejects unknown keys");
        assert!(build(name, "[1]").is_err(), "{name} rejects a non-object");
        assert!(build(name, "[{}, {}]").is_err(), "{name} rejects two objects");
    }

    let accepted = [
        ("long-function", r#"[{"maxStatements": 1, "maxLines": 1}]"#),
        ("long-function", r#"[{"maxStatements": 2.0}]"#),
        ("long-parameter-list", r#"[{"max": 0}]"#),
        ("loops", r#"[{"includeWhile": true}]"#),
        ("message-chains", r#"[{"maxDepth": 1, "ignoreRoots": [], "ignorePascalCaseRoots": false}]"#),
        ("mysterious-name", r#"[{"verbs": [], "vagueNouns": ["X"], "allowNames": [], "checkAliases": false}]"#),
        ("repeated-switches", r#"[{"minOccurrences": 2, "minSharedCases": 1}]"#),
    ];

    for (rule, options) in accepted {
        let name = format!("anti-slop-martinfowler/{rule}");

        assert!(build(&name, options).is_ok(), "{name} accepts {options}");
    }

    let rejected = [
        ("long-function", r#"[{"maxStatements": 0}]"#),
        ("long-function", r#"[{"maxLines": 1.5}]"#),
        ("long-function", r#"[{"maxLines": "80"}]"#),
        ("long-parameter-list", r#"[{"max": -1}]"#),
        ("loops", r#"[{"includeWhile": "yes"}]"#),
        ("message-chains", r#"[{"maxDepth": 0}]"#),
        ("message-chains", r#"[{"ignoreRoots": "window"}]"#),
        ("message-chains", r#"[{"ignoreRoots": [1]}]"#),
        ("message-chains", r#"[{"ignorePascalCaseRoots": null}]"#),
        ("mysterious-name", r#"[{"verbs": [1]}]"#),
        ("mysterious-name", r#"[{"checkAliases": "no"}]"#),
        ("repeated-switches", r#"[{"minOccurrences": 1}]"#),
        ("repeated-switches", r#"[{"minSharedCases": 0}]"#),
    ];

    for (rule, options) in rejected {
        let name = format!("anti-slop-martinfowler/{rule}");

        assert!(build(&name, options).is_err(), "{name} rejects {options}");
    }
}
