//! `@nkzw/eslint-plugin` 2.0.0's `test.js`, plus parenthesis and ordering cases.

mod support;

use support::{messages, only};

fn check(rule: &str, valid: &[&str], invalid: &[(&str, &str)]) {
    let linter = only(rule, &[]);

    for code in valid {
        assert_eq!(messages(&linter, "a.tsx", code), Vec::<String>::new(), "valid: {code}");
    }

    for (code, message) in invalid {
        assert_eq!(messages(&linter, "a.tsx", code), [*message], "invalid: {code}");
    }
}

#[test]
fn no_instanceof() {
    check(
        "@nkzw/no-instanceof",
        &[
            "if (value instanceof Error) {}",
            "if (value instanceof CustomError) {}",
            "if (value instanceof Exception) {}",
            "if (value instanceof CustomException) {}",
            "if (value instanceof (TypeError)) {}",
        ],
        &[
            ("if (value instanceof CustomClass) {}", "The \"instanceof\" operator is not allowed."),
            ("if (value instanceof errors.TypeError) {}", "The \"instanceof\" operator is not allowed."),
        ],
    );
}

#[test]
fn require_use_effect_arguments() {
    check(
        "@nkzw/require-use-effect-arguments",
        &[
            "import {useEffect} from 'react'; useEffect(() => {}, []);",
            "import {useEffect} from 'react'; useEffect(() => {}, [a]);",
            "import {useEffect} from 'react'; useEffect(() => {}, undefined);",
            "import {useEffect as uE} from 'react'; uE(() => {}, undefined);",
            "import R from 'react'; R.useEffect(() => {}, undefined);",
            "useEffect(() => {});",
            "useEffect(() => {}); import {useEffect} from 'react';",
            "import {useEffect} from 'preact'; useEffect(() => {});",
            "import * as R from 'react'; R.useEffect(() => {});",
        ],
        &[
            ("import {useEffect} from 'react'; useEffect(() => {});", "useEffect must be called with a second argument (dependency array)."),
            ("import {useEffect as uE} from 'react'; uE(() => {});", "uE must be called with a second argument (dependency array)."),
            ("import R from 'react'; R.useEffect(() => {});", "R.useEffect must be called with a second argument (dependency array)."),
            ("import R from 'react'; R[useEffect]();", "R.useEffect must be called with a second argument (dependency array)."),
            ("import {useEffect} from 'react'; (useEffect)(() => {});", "useEffect must be called with a second argument (dependency array)."),
        ],
    );
}

#[test]
fn ensure_relay_types() {
    check(
        "@nkzw/ensure-relay-types",
        &[
            "import { useMutation } from 'react-relay/hooks.js'; useMutation<T>(mutation, options);",
            "import { usePaginationFragment } from 'react-relay/hooks.js'; usePaginationFragment<T>(fragment, options);",
            "useMutation(mutation, options);",
            "import { useMutation } from 'react-relay'; useMutation(mutation, options);",
        ],
        &[
            ("import { useMutation } from 'react-relay/hooks.js'; useMutation(mutation, options);", "`useMutation` calls must have type parameters."),
            ("import { usePaginationFragment as pf } from 'react-relay/hooks.js'; pf(fragment, options);", "`pf` calls must have type parameters."),
        ],
    );
}

#[test]
fn rules_take_no_options() {
    for rule in ["@nkzw/no-instanceof", "@nkzw/require-use-effect-arguments", "@nkzw/ensure-relay-types"] {
        let config = fmtkit_config::Lint { bundled: false, rules: [(rule.to_owned(), serde_json::json!(["error", {}]))].into(), ignore: Vec::new() };

        assert!(fmtkit_lint::Linter::new(&config).is_err(), "{rule}");
    }
}

#[test]
fn disable_directives_apply_to_native_rules() {
    let linter = only("@nkzw/no-instanceof", &[]);
    let code = "// eslint-disable-next-line @nkzw/no-instanceof\na instanceof B;\na instanceof C; // eslint-disable-line\n/* eslint-disable */\na instanceof D;\n";

    assert_eq!(messages(&linter, "a.ts", code), Vec::<String>::new());
    assert_eq!(messages(&linter, "a.ts", "// eslint-disable-next-line eqeqeq\na instanceof B;\n").len(), 1);
}
