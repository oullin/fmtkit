//! `eslint-plugin-no-only-tests` 3.4.0's `test/unit.mjs`, plus ESTree-shape cases.

mod support;

use serde_json::{Value, json};
use support::{messages, only, output};

const RULE: &str = "no-only-tests/no-only-tests";

fn options(options: &Value) -> Vec<Value> {
    if options.is_null() { Vec::new() } else { vec![options.clone()] }
}

#[test]
fn valid() {
    let cases: &[(Value, &str)] = &[
        (Value::Null, r#"describe("Some describe block", function() {});"#),
        (Value::Null, r#"it("Some assertion", function() {});"#),
        (Value::Null, r#"xit.only("Some assertion", function() {});"#),
        (Value::Null, r#"xdescribe.only("Some describe block", function() {});"#),
        (Value::Null, r#"xcontext.only("A context block", function() {});"#),
        (Value::Null, r#"xtape.only("A tape block", function() {});"#),
        (Value::Null, r#"xtest.only("A test block", function() {});"#),
        (Value::Null, r#"other.only("An other block", function() {});"#),
        (Value::Null, r#"testResource.only("A test resource block", function() {});"#),
        (Value::Null, r#"var args = {only: "test"};"#),
        (Value::Null, r#"it("should pass meta only through", function() {});"#),
        (Value::Null, r#"obscureTestBlock.only("An obscure testing library test works unless options are supplied", function() {});"#),
        (json!({ "block": ["it"] }), r#"test.only("Options will exclude this from being caught", function() {});"#),
        (json!({ "focus": ["focus"] }), r#"test.only("Options will exclude this from being caught", function() {});"#),
        (json!({ "functions": ["fit", "xit"] }), r#"it("Options will exclude this from being caught", function() {});"#),
        // ESTree shapes oxc represents differently.
        (Value::Null, r#"describe[only]("computed", function() {});"#),
        (Value::Null, "only();"),
    ];

    for (opts, code) in cases {
        let linter = only(RULE, &options(opts));

        assert_eq!(messages(&linter, "a.js", code), Vec::<String>::new(), "{code}");
    }
}

#[test]
fn invalid() {
    assert_invalid(&[
        (Value::Null, r#"describe.only("Some describe block", function() {});"#, "describe.only not permitted", None),
        (Value::Null, r#"it.only("Some assertion", function() {});"#, "it.only not permitted", None),
        (Value::Null, r#"context.only("Some context", function() {});"#, "context.only not permitted", None),
        (Value::Null, r#"test.only("Some test", function() {});"#, "test.only not permitted", None),
        (Value::Null, r#"tape.only("A tape", function() {});"#, "tape.only not permitted", None),
        (Value::Null, r#"fixture.only("A fixture", function() {});"#, "fixture.only not permitted", None),
        (Value::Null, r#"serial.only("A serial test", function() {});"#, "serial.only not permitted", None),
        (
            json!({ "block": ["obscureTestBlock"] }),
            r#"obscureTestBlock.only("An obscure testing library test", function() {});"#,
            "obscureTestBlock.only not permitted",
            None,
        ),
        (json!({ "block": ["ava.default"] }), r#"ava.default.only("Block with dot", function() {});"#, "ava.default.only not permitted", None),
        (Value::Null, r#"it.default.before(console.log).only("Some describe block", function() {});"#, "it.default.before.only not permitted", None),
        (json!({ "focus": ["focus"] }), r#"test.focus("An alternative focus function", function() {});"#, "test.focus not permitted", None),
        (json!({ "block": ["test*"] }), r#"testResource.only("A test resource block", function() {});"#, "testResource.only not permitted", None),
        (json!({ "functions": ["fit", "xit"] }), r#"xit("No skipped tests", function() {});"#, "xit not permitted", None),
        // ESTree shapes oxc represents differently.
        (Value::Null, r#"(describe).only("parenthesized", function() {});"#, "describe.only not permitted", None),
        (Value::Null, r#"describe?.only("optional", function() {});"#, "describe.only not permitted", None),
        (json!({ "block": ["*"] }), r#"describe[only]("computed", function() {});"#, " not permitted", None),
    ]);
}

#[test]
fn invalid_with_fix() {
    assert_invalid(&[
        (
            json!({ "fix": true }),
            r#"describe.only("Some describe block", function() {});"#,
            "describe.only not permitted",
            Some(r#"describe("Some describe block", function() {});"#),
        ),
        (json!({ "fix": true }), r#"it.only("Some assertion", function() {});"#, "it.only not permitted", Some(r#"it("Some assertion", function() {});"#)),
        (
            json!({ "fix": true }),
            r#"context.only("Some context", function() {});"#,
            "context.only not permitted",
            Some(r#"context("Some context", function() {});"#),
        ),
        (json!({ "fix": true }), r#"test.only("Some test", function() {});"#, "test.only not permitted", Some(r#"test("Some test", function() {});"#)),
        (json!({ "fix": true }), r#"tape.only("A tape", function() {});"#, "tape.only not permitted", Some(r#"tape("A tape", function() {});"#)),
        (json!({ "fix": true }), r#"fixture.only("A fixture", function() {});"#, "fixture.only not permitted", Some(r#"fixture("A fixture", function() {});"#)),
        (
            json!({ "fix": true }),
            r#"serial.only("A serial test", function() {});"#,
            "serial.only not permitted",
            Some(r#"serial("A serial test", function() {});"#),
        ),
        (
            json!({ "block": ["obscureTestBlock"], "fix": true }),
            r#"obscureTestBlock.only("An obscure testing library test", function() {});"#,
            "obscureTestBlock.only not permitted",
            Some(r#"obscureTestBlock("An obscure testing library test", function() {});"#),
        ),
        (
            json!({ "block": ["ava.default"], "fix": true }),
            r#"ava.default.only("Block with dot", function() {});"#,
            "ava.default.only not permitted",
            Some(r#"ava.default("Block with dot", function() {});"#),
        ),
        (
            json!({ "fix": true }),
            r#"it.default.before(console.log).only("Some describe block", function() {});"#,
            "it.default.before.only not permitted",
            Some(r#"it.default.before(console.log)("Some describe block", function() {});"#),
        ),
        (
            json!({ "focus": ["focus"], "fix": true }),
            r#"test.focus("An alternative focus function", function() {});"#,
            "test.focus not permitted",
            Some(r#"test("An alternative focus function", function() {});"#),
        ),
        (
            json!({ "fix": true }),
            r#"Feature.only("Some Feature", function() {});"#,
            "Feature.only not permitted",
            Some(r#"Feature("Some Feature", function() {});"#),
        ),
        (
            json!({ "fix": true }),
            r#"Scenario.only("Some Scenario", function() {});"#,
            "Scenario.only not permitted",
            Some(r#"Scenario("Some Scenario", function() {});"#),
        ),
        (
            json!({ "fix": true }),
            r#"Given.only("Some assertion", function() {});"#,
            "Given.only not permitted",
            Some(r#"Given("Some assertion", function() {});"#),
        ),
        (json!({ "fix": true }), r#"And.only("Some assertion", function() {});"#, "And.only not permitted", Some(r#"And("Some assertion", function() {});"#)),
        (
            json!({ "fix": true }),
            r#"When.only("Some assertion", function() {});"#,
            "When.only not permitted",
            Some(r#"When("Some assertion", function() {});"#),
        ),
        (
            json!({ "fix": true }),
            r#"Then.only("Some assertion", function() {});"#,
            "Then.only not permitted",
            Some(r#"Then("Some assertion", function() {});"#),
        ),
        (json!({ "functions": ["fit", "xit"], "fix": true }), r#"xit("No skipped tests", function() {});"#, "xit not permitted", None),
    ]);
}

fn assert_invalid(cases: &[(Value, &str, &str, Option<&str>)]) {
    for (opts, code, message, fixed) in cases {
        let linter = only(RULE, &options(opts));

        assert_eq!(messages(&linter, "a.js", code), [*message], "{code}");
        assert_eq!(output(&linter, "a.js", code), fixed.unwrap_or(code), "{code}");
    }
}

#[test]
fn options_follow_the_schema() {
    for bad in [json!("x"), json!({ "block": "it" }), json!({ "block": ["it", "it"] }), json!({ "fix": 1 }), json!({ "other": true })] {
        let config = fmtkit_config::Lint { bundled: false, rules: [(RULE.to_owned(), json!(["error", bad]))].into(), ignore: Vec::new() };

        assert!(fmtkit_lint::Linter::new(&config).is_err(), "{bad}");
    }
}
