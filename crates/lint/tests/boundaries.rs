//! Contracts ported from oc's rules and specifier cases, through the public linter.

mod support;

use fmtkit_config::Lint;
use fmtkit_lint::Linter;
use serde_json::{Value, json};
use support::{lint, only};

const ANY: &str = "explicit any is forbidden; use a precise type or parse boundary input from unknown";
const TYPEOF: &str = "runtime typeof validation is forbidden; parse untrusted data with a zod DTO";
const ZOD: &str = "z.infer, z.input, and z.output are forbidden; write the boundary type explicitly";
const INFER: &str = "type-only infer imports are forbidden; write the boundary type explicitly";
const NULLABLE: &str = "null and undefined unions are forbidden; use the repository nullable types";
const REFLECT: &str = "dynamic property access is forbidden; parse a zod DTO";
const BARREL: &str = "index.ts barrels may contain only import and export statements";
const MODULES: &str = include_str!("fixtures/boundaries/modules.ts.txt");
const TYPES: &str = include_str!("fixtures/boundaries/types.ts.txt");

fn assert_findings(rule: &str, options: &[Value], file: &str, code: &str, expected: &[(u32, &str)]) {
    let linter = only(&format!("boundaries/{rule}"), options);
    let result = lint(&linter, file, code, true);
    let got: Vec<_> = result.diagnostics.iter().map(|diagnostic| (diagnostic.line, diagnostic.message.as_str())).collect();

    assert_eq!(got, expected, "{rule}: {file}\n{code}");
    assert!(result.fixed.is_none(), "boundary rules never rewrite source");
    assert!(result.diagnostics.iter().all(|diagnostic| diagnostic.rule.starts_with("boundaries/")), "{result:?}");
}

#[test]
fn reference_syntax_fixtures() {
    assert_findings(
        "relative-module",
        &[],
        "packages/kernel/src/example.ts",
        MODULES,
        &[
            (1, "relative module specifier is forbidden: ./value"),
            (2, "relative module specifier is forbidden: ../barrel"),
            (3, "relative module specifier is forbidden: ./named"),
            (4, "relative module specifier is forbidden: ./a"),
            (5, "relative module specifier is forbidden: ./b"),
            (6, "relative module specifier is forbidden: ./c"),
            (11, "relative module specifier is forbidden: ./literal"),
            (12, "relative module specifier is forbidden: ../literal"),
        ],
    );
    assert_findings("explicit-any", &[], "src/example.ts", TYPES, &[(1, ANY)]);
    assert_findings("runtime-typeof", &[], "src/example.ts", TYPES, &[(3, TYPEOF)]);
    assert_findings("zod-infer", &[], "src/example.ts", TYPES, &[(5, INFER), (6, ZOD), (7, ZOD), (8, ZOD), (11, INFER)]);
    assert_findings("nullable-union", &[], "src/example.ts", TYPES, &[(15, NULLABLE), (16, NULLABLE), (18, NULLABLE), (20, NULLABLE)]);
    assert_findings(
        "dynamic-property-access",
        &[],
        "src/example.ts",
        include_str!("fixtures/boundaries/reflect.ts.txt"),
        &[(1, REFLECT), (2, REFLECT), (7, REFLECT), (8, REFLECT)],
    );
    assert_findings(
        "relative-module",
        &[],
        "src/component.tsx",
        include_str!("fixtures/boundaries/component.tsx.txt"),
        &[(1, "relative module specifier is forbidden: ./view")],
    );
}

#[test]
fn layout_counts_only_top_level_exported_class_declarations() {
    let source = include_str!("fixtures/boundaries/classes.ts.txt");

    assert_findings(
        "exported-class-count",
        &[],
        "apps/api/src/example.ts",
        source,
        &[(4, "apps/api/src/example.ts exports 3 classes; one exported class per file is permitted")],
    );
    assert_findings("exported-class-count", &[], "apps/api/src/index.ts", source, &[]);
    assert_findings("exported-class-count", &[], "src/example.ts", "export class Single {}\nclass Local {}\nexport { Local };", &[]);
    assert_findings("barrel-only", &[], "apps/api/src/index.ts", include_str!("fixtures/boundaries/barrel.ts.txt"), &[(7, BARREL)]);

    for source in ["import x from '@x'; export { x }; export * from '@y';", "export default value;", "export = value;", "export import x = require('@x');", ""]
    {
        assert_findings("barrel-only", &[], "src/index.ts", source, &[]);
    }
    for source in ["export const value = 1;", "export default class {}", "export default function f() {}", "'use strict';", ";"] {
        assert_findings("barrel-only", &[], "src/index.ts", source, &[(1, BARREL)]);
        assert_findings("barrel-only", &[], "src/index.tsx", source, &[]);
        assert_findings("barrel-only", &[], "index.ts", source, &[]);
    }
}

#[test]
fn reserved_names_cover_identifiers_without_counting_strings_or_private_fields() {
    let source = include_str!("fixtures/boundaries/names.tsx.txt");
    let linter = only("boundaries/reserved-internal-name", &[json!({"names": ["lifecycle"]})]);
    let result = lint(&linter, "src/order-LIFECYCLE-service.tsx", source, false);
    let got: Vec<_> = result.diagnostics.iter().map(|diagnostic| (diagnostic.line, diagnostic.column)).collect();

    assert_eq!(
        got,
        [
            (1, 1),
            (1, 14),
            (2, 7),
            (2, 21),
            (3, 21),
            (4, 9),
            (4, 20),
            (4, 39),
            (5, 1),
            (5, 11),
            (6, 1),
            (7, 6),
            (7, 20),
            (7, 40),
            (7, 51),
            (9, 18),
            (9, 28),
            (9, 51),
            (9, 65),
            (10, 1),
            (10, 34),
            (11, 7)
        ]
    );
    assert_eq!(result.diagnostics[0].message, "internal file names must omit lifecycle");
    assert!(result.diagnostics[1..].iter().all(|diagnostic| diagnostic.message == "internal identifiers must omit lifecycle"));
    assert_findings("reserved-internal-name", &[json!({"names": ["lifecycle"]})], "src/order-state-machine.ts", "export class OrderStateMachine {}", &[]);
    assert_findings(
        "reserved-internal-name",
        &[json!({"names": ["Cycle", "Life"]})],
        "src/example.ts",
        "const LIFECYCLE = 1;",
        &[(1, "internal identifiers must omit Cycle"), (1, "internal identifiers must omit Life")],
    );
}

#[test]
fn constructors_are_keywords_and_only_authored_typescript_is_checked() {
    assert_findings("reserved-internal-name", &[json!({"names": ["constructor"]})], "src/example.ts", "class Example { constructor() {} }", &[]);
    assert_findings(
        "reserved-internal-name",
        &[json!({"names": ["constructor"]})],
        "src/example.ts",
        "const object = { constructor() {} };",
        &[(1, "internal identifiers must omit constructor")],
    );
    assert_findings("explicit-any", &[], "src/types.d.ts", "type Unsafe = any;", &[]);
    for file in ["src/types.d.ts", "src/example.js"] {
        assert_findings("relative-module", &[], file, "import value from './value';", &[]);
    }
}

#[test]
fn named_specifier_findings_keep_severity_and_token_backed_suppression() {
    let linter = Linter::new(&Lint {
        bundled: false,
        rules: [("boundaries/reserved-specifier".to_owned(), json!(["warn", specifier_options()]))].into(),
        ignore: Vec::new(),
    })
    .unwrap();
    let code = "const text = 'eslint-disable boundaries/reserved-specifier';\nimport value from 'effect';\n// eslint-disable-next-line boundaries/reserved-specifier\nimport next from 'effect';";
    let result = lint(&linter, "src/example.ts", code, false);

    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].line, 2);
    assert_eq!(result.diagnostics[0].rule, "boundaries/reserved-specifier/effect");
    assert_eq!(result.diagnostics[0].severity, fmtkit_core::Severity::Warning);
}

#[test]
fn ordered_exemptions_are_scoped_to_each_rule() {
    let options = [
        json!({"exempt": ["packages/*/src/contracts/**", "!packages/workspace/src/contracts/**", "packages/workspace/src/contracts/generated/**", "**/*.test.ts", "**/*-dto.ts", "living/src/**/dto/**"]}),
    ];

    for file in [
        "packages/auth/src/contracts/a.ts",
        "packages/workspace/src/contracts/generated/a.ts",
        "a.test.ts",
        "src/a-dto.ts",
        "living/src/dto/a.ts",
        "living/src/booking/dto/a.ts",
    ] {
        assert_findings("nullable-union", &options, file, "type Value = string | undefined;", &[]);
    }
    for file in ["packages/workspace/src/contracts/a.ts", "packages/auth/src/service.ts", "database/src/contracts/a.ts"] {
        assert_findings("nullable-union", &options, file, "type Value = string | undefined;", &[(1, NULLABLE)]);
    }
    let linter = Linter::new(&Lint {
        bundled: false,
        rules: [("boundaries/nullable-union".to_owned(), json!(["error", {"exempt": ["src/**"]}])), ("boundaries/explicit-any".to_owned(), json!("error"))]
            .into(),
        ignore: Vec::new(),
    })
    .unwrap();

    assert_eq!(support::messages(&linter, "src/example.ts", "type Value = any | null;"), [ANY]);
    assert_findings(
        "relative-module",
        &[json!({"exempt": ["packages/kernel/jest-reanimated-mock.ts"]})],
        "packages/kernel/jest-reanimated-mock.ts",
        "export * from './impl';",
        &[],
    );
    assert_findings(
        "relative-module",
        &[json!({"exempt": ["packages/kernel/jest-reanimated-mock.ts"]})],
        "packages/kernel/jest-setup.ts",
        "import mock from './mock';",
        &[(1, "relative module specifier is forbidden: ./mock")],
    );
}

fn specifier_options() -> Value {
    json!({"rules": serde_json::from_str::<Value>(include_str!("fixtures/boundaries/specifiers.json.txt")).unwrap()})
}

#[test]
fn reference_specifiers_reserve_each_module_for_its_files() {
    let cases = [
        ("framework-cache", "packages/cache/src/index.ts", "@omniyat-collection/framework/cache/memory"),
        ("framework-container", "packages/container/src/application.ts", "@omniyat-collection/framework/container"),
        ("framework-httpx", "packages/http/src/index.ts", "@omniyat-collection/framework/httpx/response"),
        ("framework-money", "packages/kernel/src/money/money.ts", "@omniyat-collection/framework/support/money"),
        ("framework-navigator", "packages/kernel/src/routes/route.ts", "@omniyat-collection/framework/httpx/routing/navigator"),
        ("framework-phone", "packages/auth/src/client/phone/international-phone-number.ts", "@omniyat-collection/framework/support/phone/mobile"),
        ("framework-workflow", "packages/workflow/src/state-definition.ts", "@omniyat-collection/framework/workflow"),
        ("better-result", "packages/primitives/src/result.ts", "better-result"),
        ("effect", "packages/primitives/src/effect/interop.ts", "effect/Effect"),
        ("ai-sdk", "packages/assistant/src/model/cloudflare-gateway.ts", "@ai-sdk/openai"),
        ("twilio", "packages/providers/src/worker-providers.ts", "@twilio"),
    ];
    let options = specifier_options();
    let linter = only("boundaries/reserved-specifier", std::slice::from_ref(&options));

    for (name, allowed, module) in cases {
        let code = format!("import {{ value }} from '{module}';");

        assert!(support::messages(&linter, allowed, &code).is_empty(), "{allowed}: {module}");

        let result = lint(&linter, "apps/api/src/service.ts", &code, false);
        let entry = options["rules"].as_array().unwrap().iter().find(|rule| rule["name"] == name).unwrap();

        assert_eq!(result.diagnostics.len(), 1, "{module}: {result:?}");
        assert_eq!(result.diagnostics[0].rule, format!("boundaries/reserved-specifier/{name}"));
        assert_eq!(result.diagnostics[0].message, entry["message"].as_str().unwrap());
    }
}

#[test]
fn specifier_exceptions_lookalikes_and_ordered_allow_paths() {
    let cases = [
        ("packages/primitives/src/nullable/nullable.ts", "better-result", Some("better-result")),
        ("packages/primitives/src/nullable/nullable.ts", "effect", Some("effect")),
        ("apps/api/src/service.ts", "airtable", None),
        ("apps/api/src/service.ts", "effection", None),
        ("apps/api/src/service.ts", "@effection/core", None),
        ("apps/api/src/service.ts", "better-results", None),
        ("apps/api/src/service.ts", "twilio-lookalike", None),
        ("apps/api/src/service.ts", "ai", Some("ai-sdk")),
        ("apps/api/src/service.ts", "@ai-sdk/provider-utils", Some("ai-sdk")),
        ("apps/api/src/service.ts", "@effect/platform", Some("effect")),
        ("packages/kernel/src/routes/route.ts", "@omniyat-collection/framework/httpx/routing/navigator", None),
        ("packages/kernel/src/routes/route.ts", "@omniyat-collection/framework/httpx/routing/navigator/child", None),
        ("packages/testing-faker/src/faker.ts", "@omniyat-collection/framework/support/phone", None),
        ("packages/testing-faker/src/faker.test.ts", "@omniyat-collection/framework/support/phone", None),
        ("packages/auth/src/otp/service.ts", "@omniyat/twilio", Some("twilio")),
        ("packages/notifications/src/messaging/dispatch.ts", "@omniyat/twilio", Some("twilio")),
        ("packages/auth/src/otp/service.test.ts", "twilio", None),
        ("packages/twilio/src/callback.ts", "@db", Some("twilio-persistence")),
        ("packages/twilio/src/callback.test.ts", "drizzle-orm", None),
        ("apps/api/src/service.ts", "@db", None),
    ];
    let linter = only("boundaries/reserved-specifier", &[specifier_options()]);

    for (file, module, expected) in cases {
        let result = lint(&linter, file, &format!("import {{ value }} from '{module}';"), false);
        let codes: Vec<_> = result.diagnostics.iter().map(|diagnostic| diagnostic.rule.as_str()).collect();
        let expected: Vec<_> = expected.map(|name| format!("boundaries/reserved-specifier/{name}")).into_iter().collect();

        assert_eq!(codes, expected, "{file}: {module}");
    }
}

#[test]
fn specifiers_share_import_extraction_and_honour_global_exemptions() {
    let options = json!({"rules": [{"name": "private-module", "match": ["./value", "../barrel", "./named", "./a", "./b", "./c", "./literal", "../literal"], "message": "reserved module"}]});

    assert_findings(
        "reserved-specifier",
        &[options],
        "src/example.ts",
        MODULES,
        &[
            (1, "reserved module"),
            (2, "reserved module"),
            (3, "reserved module"),
            (4, "reserved module"),
            (5, "reserved module"),
            (6, "reserved module"),
            (11, "reserved module"),
            (12, "reserved module"),
        ],
    );
    let mut options = specifier_options();
    options["exempt"] = json!(["src/**"]);
    assert_findings("reserved-specifier", &[options], "src/example.ts", "import { Effect } from 'effect';", &[]);
}

#[test]
fn options_reject_unknown_keys_wrong_types_and_unexpanded_packages() {
    for (name, _) in fmtkit_lint::rules::boundaries::RULES {
        let base = match *name {
            "boundaries/reserved-internal-name" => json!({"names": ["lifecycle"]}),
            "boundaries/reserved-specifier" => specifier_options(),
            _ => json!({}),
        };
        for invalid in [
            json!(null),
            json!(false),
            json!({"unknown": []}),
            json!({"exempt": "src/**"}),
            json!({"exempt": [1]}),
            json!({"exempt": [""]}),
            json!({"exempt": ["!"]}),
            json!({"exempt": ["["]}),
            json!({"exempt": ["{package}/src/**"]}),
        ] {
            let options = if let Some(object) = invalid.as_object() {
                let mut merged = base.clone();
                merged.as_object_mut().unwrap().extend(object.clone());
                merged
            } else {
                invalid
            };
            assert_invalid(name, vec![options]);
        }
        assert_invalid(name, vec![base.clone(), base]);
    }
    assert_invalid("boundaries/reserved-internal-name", vec![]);
    assert_invalid("boundaries/reserved-internal-name", vec![json!({"names": []})]);
    assert_invalid("boundaries/relative-module", vec![json!({"names": ["lifecycle"]})]);
    assert_invalid("boundaries/relative-module", vec![json!({"allowlist": []})]);
    assert_invalid("boundaries/relative-module", vec![json!({"frozen": true})]);
    for invalid in [json!({}), json!({"rules": []}), json!({"rules": "private"}), json!({"rules": [1]})] {
        assert_invalid("boundaries/reserved-specifier", vec![invalid]);
    }
    for patch in [
        json!({"name": ""}),
        json!({"name": "explicit-any"}),
        json!({"name": "registered-workspace"}),
        json!({"message": " "}),
        json!({"match": []}),
        json!({"match": [""]}),
        json!({"except": [1]}),
        json!({"except": ["!"]}),
        json!({"allow": ["["]}),
        json!({"allow": ["{package}/**"]}),
        json!({"unknown": true}),
    ] {
        let mut rule = json!({"name": "private", "match": ["effect"], "message": "private module"});
        rule.as_object_mut().unwrap().extend(patch.as_object().unwrap().clone());
        assert_invalid("boundaries/reserved-specifier", vec![json!({"rules": [rule]})]);
    }
    let duplicate = json!({"name": "private", "match": ["effect"], "message": "private module"});
    assert_invalid("boundaries/reserved-specifier", vec![json!({"rules": [duplicate, duplicate]})]);
}

fn assert_invalid(name: &str, options: Vec<Value>) {
    let mut setting = vec![json!("error")];
    setting.extend(options);
    let config = Lint { bundled: false, rules: [(name.to_owned(), json!(setting))].into(), ignore: Vec::new() };
    let error = Linter::new(&config).err().unwrap_or_else(|| panic!("accepted {name}: {setting:?}"));

    assert!(error.to_string().contains(name), "{error}");
    if json!(setting).to_string().contains("{package}") {
        assert!(error.to_string().contains("{package} is not supported"), "{error}");
    }
}
