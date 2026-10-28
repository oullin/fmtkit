//! Catch policy contracts, including every source extractor fixture unchanged.

mod support;

use fmtkit_config::Lint;
use fmtkit_lint::Linter;
use serde_json::{Value, json};
use support::{lint, only};

const BOUNDARY: &str = "observability/no-catch-outside-boundary";
const BOUNDARY_MESSAGE: &str = "A catch outside a failure boundary: raise an OperationalFailure and let the edge observe it.";
const CLAUSE: &str = "observability/no-unobserved-catch";
const PROMISE: &str = "observability/no-swallowing-promise-catch";
const CLAUSE_MESSAGE: &str = "Catch must report, throw, or carry the caught error into an outcome with errorClass";
const PROMISE_MESSAGE: &str = "Promise catch handler must not be empty or return undefined";

type FindingSite = (&'static str, u32, u32);
type Fixture = (&'static str, &'static str, &'static [FindingSite]);

#[test]
fn reference_fixtures() {
    let linter =
        Linter::new(&Lint { bundled: false, rules: [(CLAUSE.to_owned(), json!("error")), (PROMISE.to_owned(), json!("error"))].into(), ignore: Vec::new() })
            .unwrap();
    let fixtures: &[Fixture] = &[
        (
            "clauses.ts",
            include_str!("fixtures/observability/clauses.ts.txt"),
            &[
                (CLAUSE, 2, 20),
                (CLAUSE, 5, 20),
                (CLAUSE, 6, 20),
                (CLAUSE, 9, 20),
                (CLAUSE, 15, 20),
                (CLAUSE, 19, 20),
                (CLAUSE, 22, 20),
                (CLAUSE, 25, 20),
                (CLAUSE, 28, 20),
                (CLAUSE, 34, 20),
                (CLAUSE, 56, 20),
            ],
        ),
        ("component.tsx", include_str!("fixtures/observability/component.tsx.txt"), &[(CLAUSE, 3, 20)]),
        (
            "escapes.ts",
            include_str!("fixtures/observability/escapes.ts.txt"),
            &[(CLAUSE, 10, 2), (CLAUSE, 14, 2), (CLAUSE, 15, 20), (CLAUSE, 20, 2), (CLAUSE, 23, 5), (CLAUSE, 26, 6), (PROMISE, 37, 9), (PROMISE, 40, 10)],
        ),
        (
            "promises.ts",
            include_str!("fixtures/observability/promises.ts.txt"),
            &[
                (PROMISE, 1, 9),
                (PROMISE, 2, 9),
                (PROMISE, 3, 9),
                (PROMISE, 4, 9),
                (PROMISE, 5, 9),
                (PROMISE, 6, 9),
                (PROMISE, 7, 9),
                (PROMISE, 8, 9),
                (PROMISE, 9, 9),
                (PROMISE, 10, 9),
                (PROMISE, 11, 9),
                (PROMISE, 12, 9),
                (PROMISE, 29, 9),
                (PROMISE, 30, 9),
                (PROMISE, 31, 9),
                (PROMISE, 32, 9),
                (PROMISE, 33, 9),
            ],
        ),
    ];

    for (file, code, expected) in fixtures {
        let result = lint(&linter, file, code, false);
        let actual: Vec<_> = result.diagnostics.iter().map(|d| (d.rule.as_str(), d.line, d.column)).collect();

        assert_eq!(actual, *expected, "{file}");

        for diagnostic in &result.diagnostics {
            assert_eq!(diagnostic.message, if diagnostic.rule == CLAUSE { CLAUSE_MESSAGE } else { PROMISE_MESSAGE });
        }
    }
}

fn assert_sites(rule: &str, options: &[Value], file: &str, code: &str, expected: &[(u32, u32)]) {
    let result = lint(&only(rule, options), file, code, false);
    let actual: Vec<_> = result.diagnostics.iter().map(|d| (d.line, d.column)).collect();

    assert_eq!(actual, expected, "{file}: {code}");
    assert!(result.diagnostics.iter().all(|d| d.rule == rule));
    assert!(result.fixed.is_none());
}

#[test]
fn clause_evidence_and_wrappers() {
    let valid = [
        "try {} catch (e) { ((reportException as Reporter)!)(e); }",
        "try {} catch (e) { ((console as Console)!).error(e); }",
        "try {} catch (e) { (Sentry satisfies Reporter)['captureException'](e); }",
        "try {} catch (e) { (<Reporter>reportException)(e); }",
        "try {} catch (e) { OperationalLogger['anything'](e); }",
        "try {} catch (e) { arbitrary.owner['reportException'](e); }",
        "try {} catch (e) { const outcome = { cause: e, errorClass: 'E' }; }",
        "try {} catch (e) { target = wrap(e, { 'errorClass': 'E' }); }",
        "function f() { try {} catch (e) { return { e, errorClass }; } }",
    ];

    for code in valid {
        assert_sites(CLAUSE, &[], "a.ts", code, &[]);
    }

    let invalid = [
        "try {} catch (e) { const outcome = { e: other, errorClass: 'E' }; }",
        "try {} catch (e) { const outcome = { cause: other.e, errorClass: 'E' }; }",
        "try {} catch (e) { const outcome = { cause: e, ['errorClass']: 'E' }; }",
        "try {} catch (e) { const outcome = { cause: e, get errorClass() { return 'E'; } }; }",
        "try {} catch (e) { target += { cause: e, errorClass: 'E' }; }",
        "try {} catch { const outcome = { cause: e, errorClass: 'E' }; }",
        "try {} catch ({ e }) { const outcome = { cause: e, errorClass: 'E' }; }",
        "try {} catch (e) { class Later { static { throw e; } } }",
        "try {} catch (e) { const outcome = { errorClass: 'E', cause: () => e }; }",
        "try {} catch (e) { const outcome = { cause: e, later: () => ({errorClass: 'E'}) }; }",
        "try {} catch (e) { const outcome = { cause: other as e, errorClass: 'E' }; }",
        "try {} catch (e) { console[method](e); }",
        "try {} catch (e) { { const e = other; const outcome = { cause: e, errorClass: 'E' }; } }",
        "try {} catch (e) { let e2; { let e = other; e2 = { cause: e, errorClass: 'E' }; } }",
    ];

    for code in invalid {
        assert_sites(CLAUSE, &[], "a.ts", code, &[(1, 8)]);
    }
}

#[test]
fn promise_resolution_and_nesting() {
    let valid = [
        "request.catch(() => { 'use strict'; });",
        "request.catch(() => { function nested() { return; } });",
        "request.catch(() => { class Nested { method() { return; } } });",
        "request.catch(() => { try {} catch { return; } });",
        "request.catch(() => { const obj = { method() { return; } }; });",
        "const empty = () => {}; function f(empty) { request.catch(empty); }",
        "import { discard } from './elsewhere'; request.catch(discard);",
        "const a = b.x; const b = a; request.catch(a);",
        "const a = a.x; request.catch(a);",
        "function handler(undefined) { return undefined; } request.catch(handler);",
        "request.catch(() => { const undefined = fallback; return undefined; });",
        "const obj = { catch() {} }; const text = 'request.catch(() => {})';",
        "request[catchName](() => {});",
        "request.catch(); request.catch(...handlers);",
        "const discard = function () {}; const obj = { discard }; request.catch(obj.discard);",
        "let handler = () => {}; handler = (e) => { throw e; }; request.catch(handler);",
        "class Handlers { static get discard() { return undefined; } } request.catch(Handlers.discard);",
    ];

    for code in valid {
        assert_sites(PROMISE, &[], "a.ts", code, &[]);
    }

    let cases: &[(&str, &[(u32, u32)])] = &[
        ("request['catch'](() => {});", &[(1, 9)]),
        ("request.catch(() => ok ? (undefined!) : value);", &[(1, 9)]),
        ("request.catch(() => { return ok ? value : other ? void action() : value; });", &[(1, 9)]),
        ("const a = b; const b = () => {};\nrequest.catch(a);", &[(2, 9)]),
        ("const obj = { discard: () => undefined }; const alias = obj;\nrequest.catch(alias.discard);", &[(2, 9)]),
        ("const discard = function () {}; const obj = { discard: discard };\nrequest.catch(obj.discard);", &[(2, 9)]),
        ("class Handlers { discard() {} run() {\nrequest.catch(this.discard); } }", &[(2, 9)]),
        ("const obj = { discard() {}, run() {\nrequest.catch(this.discard); } };", &[(2, 9)]),
        ("class Base { discard() {} } class Child extends Base {} const child = new Child();\nrequest.catch(child.discard);", &[(2, 9)]),
        ("const obj = { inner: { discard: () => {} } };\nrequest.catch(obj.inner.discard);", &[(2, 9)]),
        ("class Handlers { static discard() {} }\nrequest.catch(Handlers.discard);", &[(2, 9)]),
        ("class Handlers { discard() {} } const handlers = new Handlers();\nrequest.catch(handlers.discard);", &[(2, 9)]),
        ("request.catch(() => { request.catch(() => undefined); });", &[(1, 31)]),
    ];

    for (code, sites) in cases {
        assert_sites(PROMISE, &[], "a.ts", code, sites);
    }
}

#[test]
fn escape_tokens_and_line_endings() {
    for newline in ["\n", "\r\n", "\r", "\u{2028}", "\u{2029}"] {
        for (rule, site) in [
            (CLAUSE, "try {} catch {}"),
            (PROMISE, "request['catch'](() => undefined);"),
            (BOUNDARY, "try {} catch {}"),
            (BOUNDARY, "request['catch'](handler);"),
        ] {
            let code = format!("\t// observability: swallowed because\t a reason{newline}{site}");

            assert_sites(rule, &[], "a.ts", &code, &[]);
        }
    }

    let cases: &[(&str, &[(u32, u32)])] = &[
        ("/*\n// observability: swallowed because fake\n*/ request.catch(() => {});", &[(3, 12)]),
        ("const s = `\n// observability: swallowed because fake\n`; request.catch(() => {});", &[(3, 12)]),
        ("action(); // observability: swallowed because trailing\nrequest.catch(() => {});", &[(2, 9)]),
        ("// observability: swallowed because \t \nrequest.catch(() => {});", &[(2, 9)]),
        ("// observability: swallowed because\u{a0}reason\nrequest.catch(() => {});", &[(2, 9)]),
        ("// observability: swallowed because a reason\n\nrequest.catch(() => {});", &[(3, 9)]),
    ];

    for (code, sites) in cases {
        assert_sites(PROMISE, &[], "a.ts", code, sites);
        assert_sites(BOUNDARY, &[], "a.ts", code, sites);
    }
}

#[test]
fn ignore_is_per_rule_and_matches_repository_relative_globs() {
    for (rule, code, column) in [(CLAUSE, "try {} catch {}", 8), (PROMISE, "request.catch(() => {});", 9)] {
        let options = [json!({ "ignore": ["src/generated/**", "**/*.test.ts"] })];

        assert_sites(rule, &options, "src/generated/a.ts", code, &[]);
        assert_sites(rule, &options, "other/a.test.ts", code, &[]);
        assert_sites(rule, &options, "src/a.ts", code, &[(1, column)]);
    }

    let linter = Linter::new(&Lint {
        bundled: false,
        rules: [(CLAUSE.to_owned(), json!(["error", { "ignore": ["src/**"] }])), (PROMISE.to_owned(), json!("error"))].into(),
        ignore: Vec::new(),
    })
    .unwrap();
    let result = lint(&linter, "src/a.ts", "try {} catch {}\nrequest.catch(() => {});", false);

    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!((&*result.diagnostics[0].rule, result.diagnostics[0].line, result.diagnostics[0].column), (PROMISE, 2, 9));
}

#[test]
fn observers_replace_defaults() {
    let options = [json!({ "observers": ["record", "Logger.write", "Telemetry.*", "*.observe"] })];

    for call in ["record(e)", "Logger.write(e)", "Telemetry.anything(e)", "some.owner['observe'](e)"] {
        assert_sites(CLAUSE, &options, "a.ts", &format!("try {{}} catch (e) {{ {call}; }}"), &[]);
    }

    for call in ["reportException(e)", "console.error(e)", "Logger.other(e)", "other.record(e)", "nested.Logger.write(e)"] {
        assert_sites(CLAUSE, &options, "a.ts", &format!("try {{}} catch (e) {{ {call}; }}"), &[(1, 8)]);
    }

    assert_sites(CLAUSE, &[json!({ "observers": [] })], "a.ts", "try {} catch (e) { throw e; }", &[]);
    assert_sites(CLAUSE, &[json!({ "observers": [] })], "a.ts", "try {} catch (e) { reportException(e); }", &[(1, 8)]);
}

#[test]
fn options_are_strict() {
    for rule in [CLAUSE, PROMISE] {
        let invalid = [
            vec![json!(null)],
            vec![json!(false)],
            vec![json!({}), json!({})],
            vec![json!({ "unknown": [] })],
            vec![json!({ "ignore": "src/**" })],
            vec![json!({ "ignore": [1] })],
            vec![json!({ "ignore": ["a", "a"] })],
            vec![json!({ "ignore": ["["] })],
        ];

        for options in invalid {
            assert_invalid_options(rule, options);
        }

        assert_sites(rule, &[json!({})], "a.ts", "const x = 1;", &[]);
    }

    for observers in [json!(null), json!("record"), json!([1]), json!(["x", "x"]), json!([""]), json!(["*.*"]), json!(["a.b.c"]), json!(["a.*x"])] {
        assert_invalid_options(CLAUSE, vec![json!({ "observers": observers })]);
    }

    assert_invalid_options(PROMISE, vec![json!({ "observers": [] })]);
}

fn assert_invalid_options(rule: &str, options: Vec<Value>) {
    let mut setting = vec![json!("error")];

    setting.extend(options);

    let config = Lint { bundled: false, rules: [(rule.to_owned(), json!(setting))].into(), ignore: Vec::new() };

    assert!(Linter::new(&config).is_err(), "{rule}: {setting:?}");
}

#[test]
fn catches_outside_boundaries() {
    let code = "try {} catch (e) { reportException(e); }\nrequest.catch(handler);\nrequest['catch'](() => {});\nrequest?.catch?.(handler);";
    let result = lint(&only(BOUNDARY, &[]), "src/domain.ts", code, true);
    let actual: Vec<_> = result.diagnostics.iter().map(|d| (d.rule.as_str(), d.message.as_str(), d.line, d.column)).collect();

    assert_eq!(
        actual,
        [(BOUNDARY, BOUNDARY_MESSAGE, 1, 8), (BOUNDARY, BOUNDARY_MESSAGE, 2, 9), (BOUNDARY, BOUNDARY_MESSAGE, 3, 9), (BOUNDARY, BOUNDARY_MESSAGE, 4, 10)]
    );
    assert!(result.fixed.is_none());
    assert_sites(BOUNDARY, &[json!({})], "src/domain.js", "try {} catch {}", &[(1, 8)]);
    assert_sites(BOUNDARY, &[], "a.ts", "request.catch(); request[method](handler); const text = 'try {} catch {}';", &[]);
    assert_sites(BOUNDARY, &[], "a.ts", "request.catch(...handlers);", &[(1, 9)]);
    // A handler reassigned after its declaration is unresolved, so its single-throw initialiser grants no translation.
    assert_sites(BOUNDARY, &[], "a.ts", "let handler = (e) => { throw e; };\nhandler = (e) => {};\nrequest.catch(handler);", &[(3, 9)]);
    assert_sites(BOUNDARY, &[], "a.ts", "const handler = (e) => { throw e; };\nrequest.catch(handler);", &[]);
}

#[test]
fn boundary_paths_use_ordered_boundary_globs() {
    let options = [
        json!({ "allow": ["packages/observability/**", "**/worker.ts", "**/*.test.ts", "!packages/observability/domain/**", "packages/observability/domain/safe.ts"] }),
    ];
    let code = "try {} catch {}\nrequest.catch(handler);";

    for file in ["packages/observability/log.ts", "worker.ts", "src/worker.ts", "a.test.ts", "src/a.test.ts", "packages/observability/domain/safe.ts"] {
        assert_sites(BOUNDARY, &options, file, code, &[]);
    }
    for file in ["src/domain.ts", "packages/observability/domain/checked.ts", "other/packages/observability/a.ts", "src/worker.tsx"] {
        assert_sites(BOUNDARY, &options, file, code, &[(1, 8), (2, 9)]);
    }

    for (pattern, allowed, checked) in [
        ("src/*.ts", "src/a.ts", "src/nested/a.ts"),
        ("worker.ts", "worker.ts", "src/worker.ts"),
        ("src/?.ts", "src/é.ts", "src/ab.ts"),
        ("src/[^a].ts", "src/é.ts", "src/a.ts"),
        ("src/[!a].ts", "src/!.ts", "src/b.ts"),
        ("src/{one,two}.ts", "src/{one,two}.ts", "src/one.ts"),
    ] {
        let options = [json!({ "allow": [pattern] })];

        assert_sites(BOUNDARY, &options, allowed, code, &[]);
        assert_sites(BOUNDARY, &options, checked, code, &[(1, 8), (2, 9)]);
    }

    assert_sites(BOUNDARY, &[json!({ "allow": [] })], "src/a.ts", code, &[(1, 8), (2, 9)]);
    assert_sites(BOUNDARY, &[json!({ "allow": ["!src/**"] })], "src/a.ts", code, &[(1, 8), (2, 9)]);
    assert_sites(BOUNDARY, &[json!({ "allow": ["**", "!src/?.ts"] })], "src/é.ts", code, &[(1, 8), (2, 9)]);
    assert_sites(BOUNDARY, &[json!({ "allow": ["**/?.ts", "!src/a.ts"] })], "src/a.ts", code, &[(1, 8), (2, 9)]);
}

#[test]
fn boundary_translation_requires_only_one_throw() {
    for code in [
        "try {} catch (e) { throw e; }",
        "try {} catch { throw new Error('translated'); }",
        "try {} catch (e) { /* translation */ throw new Error('translated', { cause: e }); }",
    ] {
        assert_sites(BOUNDARY, &[], "a.ts", code, &[]);
        assert_sites(BOUNDARY, &[json!({ "translate": false })], "a.ts", code, &[(1, 8)]);
    }
    for code in [
        "try {} catch (e) { reportException(e); throw e; }",
        "try {} catch (e) { if (e) throw e; }",
        "try {} catch (e) { { throw e; } }",
        "try {} catch (e) { ; throw e; }",
        "try {} catch (e) { function later() { throw e; } }",
        "try {} catch (e) { 'use strict'; throw e; }",
    ] {
        assert_sites(BOUNDARY, &[], "a.ts", code, &[(1, 8)]);
    }
    for (prefix, handler) in [
        ("", "e => { throw e; }"),
        ("", "function(e) { throw new Error('translated', { cause: e }); }"),
        ("function translate(e) { throw e; }\n", "translate"),
        ("const obj = { translate(e) { throw e; } };\n", "obj.translate"),
        ("const translate = e => { throw e; }; const alias = translate;\n", "alias"),
    ] {
        let code = format!("{prefix}request.catch({handler});");
        let line = if prefix.is_empty() { 1 } else { 2 };

        assert_sites(BOUNDARY, &[], "a.ts", &code, &[]);
        assert_sites(BOUNDARY, &[json!({ "translate": false })], "a.ts", &code, &[(line, 9)]);
    }
    for handler in ["e => e", "e => { 'use strict'; throw e; }", "e => { reportException(e); throw e; }", "e => { if (e) throw e; }"] {
        assert_sites(BOUNDARY, &[], "a.ts", &format!("request.catch({handler});"), &[(1, 9)]);
    }
}

#[test]
fn boundary_options_are_strict() {
    for options in [
        vec![json!(null)],
        vec![json!(false)],
        vec![json!({}), json!({})],
        vec![json!({ "unknown": [] })],
        vec![json!({ "ignore": [] })],
        vec![json!({ "allow": "src/**" })],
        vec![json!({ "allow": [1] })],
        vec![json!({ "allow": ["["] })],
        vec![json!({ "allow": [""] })],
        vec![json!({ "allow": ["! "] })],
        vec![json!({ "allow": ["{package}/**"] })],
        vec![json!({ "translate": "true" })],
        vec![json!({ "translate": null })],
    ] {
        assert_invalid_options(BOUNDARY, options);
    }
}
