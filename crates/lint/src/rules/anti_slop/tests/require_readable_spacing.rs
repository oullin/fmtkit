//! `anti-slop/require-readable-spacing`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/rules/require-readable-spacing.test.ts`.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use fmtkit_config::Lint;
use fmtkit_core::Lang;
use serde_json::json;

use super::{Case, check};
use crate::Linter;

/// src/rules/require-readable-spacing.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop/require-readable-spacing",
        "[]",
        &[
            Case {
                name: "valid 0: import { a } from 'a';",
                path: "file.ts",
                code: "import { a } from 'a';\nimport { b } from 'b';\n\nexport const c = a + b;",
                expected: &[],
            },
            Case { name: "valid 1: function f() {", path: "file.ts", code: "function f() {\nconst a = 1;\nconst b = 2;\n\nreturn a + b;\n}", expected: &[] },
            Case { name: "valid 2: function f() { return 1; }", path: "file.ts", code: "function f() { return 1; }", expected: &[] },
            Case {
                name: "valid 3: function f(a: string): string;",
                path: "file.ts",
                code: "function f(a: string): string;\nfunction f(a: number): number;\nfunction f(a: string | number) { return a; }",
                expected: &[],
            },
            Case {
                name: "valid 4: export function f(a: string): string;",
                path: "file.ts",
                code: "export function f(a: string): string;\nexport function f(a: number): number;\nexport function f(a: string | number) { return a; }",
                expected: &[],
            },
            Case {
                name: "valid 5: const f = Effect.gen(function* () {",
                path: "file.ts",
                code: "const f = Effect.gen(function* () {\nconst a = yield* A;\nconst b = yield* B;\n\nreturn a + b;\n});",
                expected: &[],
            },
            Case {
                name: "valid 6: export type A = string;",
                path: "file.ts",
                code: "export type A = string;\n\n/** B documentation. */\nexport type B = number;",
                expected: &[],
            },
            Case {
                name: "valid 7: function f() { if (ok) { go(); } else { stop(); } }",
                path: "file.ts",
                code: "function f() { if (ok) { go(); } else { stop(); } }",
                expected: &[],
            },
            Case { name: "valid 8: function f() {", path: "file.ts", code: "function f() {\n// return docs\nreturn 1;\n}", expected: &[] },
            Case { name: "valid 9: const a = 1;", path: "file.ts", code: "const a = 1;\n\n\nconst b = 2;", expected: &[] },
            Case {
                name: "valid 10: switch (x) { case 1: case 2: go(); break; default: stop(); }",
                path: "file.ts",
                code: "switch (x) { case 1: case 2: go(); break; default: stop(); }",
                expected: &[],
            },
            Case {
                name: "invalid 11: const a = 1;",
                path: "file.ts",
                code: "const a = 1;\nconst b = 2;",
                expected: &[(13, 25, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 12: export const a = 1;",
                path: "file.ts",
                code: "export const a = 1;\n/** B docs. */\nexport type B = number;",
                expected: &[(35, 58, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 13: const a = 1; // trailing",
                path: "file.ts",
                code: "const a = 1; // trailing\n// leading\nconst b = 2;",
                expected: &[(36, 48, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 14: const a = 1; const b = 2;",
                path: "file.ts",
                code: "const a = 1; const b = 2;",
                expected: &[(13, 25, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 15: import { a } from 'a';",
                path: "file.ts",
                code: "import { a } from 'a';\nconst b = a;",
                expected: &[(23, 35, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 16: function f() {",
                path: "file.ts",
                code: "function f() {\nconst a = 1;\nreturn a;\n}",
                expected: &[(28, 37, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 17: function f() {",
                path: "file.ts",
                code: "function f() {\nconst a = 1;\nif (a) go();\n}",
                expected: &[(28, 40, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 18: function f() {",
                path: "file.ts",
                code: "function f() {\nif (ok) { go(); }\nstop();\n}",
                expected: &[(33, 40, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 19: const f = Effect.gen(function* () {",
                path: "file.ts",
                code: "const f = Effect.gen(function* () {\nconst a = yield* A;\nconst b = yield* B;\nconst dispatch = Effect.fn('dispatch')(function* () {\nyield* a;\n});\nreturn dispatch;\n});",
                expected: &[(76, 129, "Expected blank line before this statement."), (144, 160, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 20: export interface A {}",
                path: "file.ts",
                code: "export interface A {}\nexport class B {}",
                expected: &[(22, 39, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 21: const a = 1",
                path: "file.ts",
                code: "const a = 1\n;[1].forEach(f)",
                expected: &[(13, 27, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 22: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo();\nwhile (ok) go();\n}",
                expected: &[(22, 38, "Expected blank line before this statement.")],
            },
        ],
    );
}

/// Edge cases the upstream tests miss, recorded from the upstream plugin under
/// oxlint 1.86 by the oracle (`require-readable-spacing.edges*.test.ts`).
#[test]
fn edges() {
    check(
        "anti-slop/require-readable-spacing",
        "[]",
        &[
            Case {
                name: "invalid 0: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo();\n{\nbar();\n}\nbaz();\n}",
                expected: &[(33, 39, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 1: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo();\nswitch (x) {\ncase 1: {\n}\ncase 2:\nbreak;\n}\nbar();\n}",
                expected: &[
                    (22, 34, "Expected blank line before this statement."),
                    (47, 54, "Expected blank line before this statement."),
                    (64, 70, "Expected blank line before this statement."),
                ],
            },
            Case {
                name: "invalid 2: switch (x) {",
                path: "file.ts",
                code: "switch (x) {\ncase 1: { a(); }\ncase 2: b();\ncase 3: if (y) { c(); }\ndefault:\nd();\n}",
                expected: &[(30, 42, "Expected blank line before this statement."), (67, 75, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 3: switch (x) {",
                path: "file.ts",
                code: "switch (x) {\ncase 1:\na();\nreturn;\n}",
                expected: &[(26, 33, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 4: \"use strict\";",
                path: "file.ts",
                code: "\"use strict\";\nconst a = 1;",
                expected: &[(14, 26, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 5: function f() {",
                path: "file.ts",
                code: "function f() {\n\"use strict\";\nreturn 1;\n}",
                expected: &[(29, 38, "Expected blank line before this statement.")],
            },
            Case { name: "invalid 6: function f() {", path: "file.ts", code: "function f() {\n\"use strict\";\nfoo();\n}", expected: &[] },
            Case { name: "invalid 7: namespace N {", path: "file.ts", code: "namespace N {\n\"a\";\nfoo();\n}", expected: &[] },
            Case { name: "invalid 8: a: b;", path: "file.ts", code: "a: b;\nc: d;", expected: &[] },
            Case {
                name: "invalid 9: foo();",
                path: "file.ts",
                code: "foo();\nlabel: for (;;) {}",
                expected: &[(7, 25, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 10: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo();\nouter: for (;;) {}\n}",
                expected: &[(22, 40, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 11: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo();\nlabel: {\n}\nbar();\n}",
                expected: &[(33, 39, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 12: function f() {",
                path: "file.ts",
                code: "function f() {\nconst a = () => {\n};\nfoo();\n}",
                expected: &[(36, 42, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 13: function f() {",
                path: "file.ts",
                code: "function f() {\nconst g = () => {};\nfoo();\n}",
                expected: &[(35, 41, "Expected blank line before this statement.")],
            },
            Case { name: "invalid 14: function f() {", path: "file.ts", code: "function f() {\nconst g = () => x;\nfoo();\n}", expected: &[] },
            Case {
                name: "invalid 15: function f() {",
                path: "file.ts",
                code: "function f() {\n(function () {})();\nfoo();\n}",
                expected: &[(35, 41, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 16: function f() {",
                path: "file.ts",
                code: "function f() {\n!function () {}();\nfoo();\n}",
                expected: &[(34, 40, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 17: function f() {",
                path: "file.ts",
                code: "function f() {\n(0, () => {})();\nfoo();\n}",
                expected: &[(32, 38, "Expected blank line before this statement.")],
            },
            Case { name: "invalid 18: function f() {", path: "file.ts", code: "function f() {\nx = class {};\nfoo();\n}", expected: &[] },
            Case {
                name: "invalid 19: function f() {",
                path: "file.ts",
                code: "function f() {\nx = function () {};\nfoo();\n}",
                expected: &[(35, 41, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 20: function f() {",
                path: "file.ts",
                code: "function f() {\nnew function () {};\nfoo();\n}",
                expected: &[(35, 41, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 21: function f() {",
                path: "file.ts",
                code: "function f() {\nx = a ? b : () => {};\nfoo();\n}",
                expected: &[(37, 43, "Expected blank line before this statement.")],
            },
            Case { name: "invalid 22: function f() {", path: "file.ts", code: "function f() {\nlet x: {};\nfoo();\n}", expected: &[] },
            Case { name: "invalid 23: function f() {", path: "file.ts", code: "function f() {\nconst o = { a: 1 };\nfoo();\n}", expected: &[] },
            Case { name: "invalid 24: function f() {", path: "file.ts", code: "function f() {\nimport('a').then(() => {});\nfoo();\n}", expected: &[] },
            Case {
                name: "invalid 25: function f() {",
                path: "file.ts",
                code: "function f() {\ntry {} catch {}\nfoo();\n}",
                expected: &[(31, 37, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 26: function f() {",
                path: "file.ts",
                code: "function f() {\ntry {} finally {}\nfoo();\n}",
                expected: &[(33, 39, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 27: function f() {",
                path: "file.ts",
                code: "function f() {\ndo {\n} while (x);\nfoo();\n}",
                expected: &[(33, 39, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 28: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo();\ndo foo(); while (x);\nbar();\n}",
                expected: &[(22, 42, "Expected blank line before this statement.")],
            },
            Case { name: "invalid 29: function f() {", path: "file.ts", code: "function f() {\nif (a) {} else b();\nfoo();\n}", expected: &[] },
            Case {
                name: "invalid 30: function f() {",
                path: "file.ts",
                code: "function f() {\nwhile (a) {}\nfoo();\n}",
                expected: &[(28, 34, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 31: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo();\nreturn () => {\n};\n}",
                expected: &[(22, 36, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 32: function f() {",
                path: "file.ts",
                code: "function f() {\nreturn function () {};\nfoo();\n}",
                expected: &[(38, 44, "Expected blank line before this statement.")],
            },
            Case { name: "invalid 33: function f() {", path: "file.ts", code: "function f() {\nthrow {};\nfoo();\n}", expected: &[] },
            Case {
                name: "invalid 34: function f() {",
                path: "file.ts",
                code: "function f() {\nlet a = 1, b = 2;\nlet c = {\n};\nvar d = 1;\nvar e = [\n];\nfoo();\n}",
                expected: &[
                    (33, 42, "Expected blank line before this statement."),
                    (46, 56, "Expected blank line before this statement."),
                    (57, 66, "Expected blank line before this statement."),
                    (70, 76, "Expected blank line before this statement."),
                ],
            },
            Case {
                name: "invalid 35: async function f() {",
                path: "file.ts",
                code: "async function f() {\nfoo();\nawait using a = b(\n);\nusing c = d;\nfoo();\n}",
                expected: &[(28, 46, "Expected blank line before this statement."), (50, 62, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 36: async function f() {",
                path: "file.ts",
                code: "async function f() {\nfoo();\nfor (const a of b) go();\nfor (const a in b) go();\nfor await (const a of b) go();\n}",
                expected: &[
                    (28, 52, "Expected blank line before this statement."),
                    (53, 77, "Expected blank line before this statement."),
                    (78, 108, "Expected blank line before this statement."),
                ],
            },
            Case {
                name: "invalid 37: function f() {",
                path: "file.ts",
                code: "function f() {\nclass A {}\nfoo();\n}",
                expected: &[(26, 32, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 38: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo();\nclass A {}\n}",
                expected: &[(22, 32, "Expected blank line before this statement.")],
            },
            Case { name: "invalid 39: function f() {", path: "file.ts", code: "function f() {\nfoo();\n@dec class A {}\n}", expected: &[] },
            Case {
                name: "invalid 40: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo();\nfunction g() {}\n}",
                expected: &[(22, 37, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 41: function f() {",
                path: "file.ts",
                code: "function f() {\nfunction g() {}\nfoo();\n}",
                expected: &[(31, 37, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 42: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo();\ntype A = string;\ninterface B {}\nenum C {}\n}",
                expected: &[
                    (22, 38, "Expected blank line before this statement."),
                    (39, 53, "Expected blank line before this statement."),
                    (54, 63, "Expected blank line before this statement."),
                ],
            },
            Case { name: "invalid 43: namespace N {", path: "file.ts", code: "namespace N {\nabstract class A {}\nfoo();\n}", expected: &[] },
            Case {
                name: "invalid 44: declare namespace N {",
                path: "file.ts",
                code: "declare namespace N {\nclass A {}\nconst b: 1;\n}",
                expected: &[(33, 44, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 45: declare namespace N {",
                path: "file.ts",
                code: "declare namespace N {\nconst a: number;\nfunction f(): void;\nfunction g(): void;\n}",
                expected: &[],
            },
            Case { name: "invalid 46: namespace N {", path: "file.ts", code: "namespace N {\nexport const a = 1;\nexport function f() {}\n}", expected: &[] },
            Case {
                name: "invalid 47: namespace N {",
                path: "file.ts",
                code: "namespace N {\nexport function f(a: string): string;\nexport function f(a: number) { return a; }\nconst b = 1;\n}",
                expected: &[(95, 107, "Expected blank line before this statement.")],
            },
            Case { name: "invalid 48: namespace N {", path: "file.ts", code: "namespace N {\nexport const a = {\n};\nexport const b = 1;\n}", expected: &[] },
            Case {
                name: "invalid 49: declare module 'x' {",
                path: "file.ts",
                code: "declare module 'x' {\nimport a from 'a';\nimport b from 'b';\nconst c: 1;\n}",
                expected: &[(59, 70, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 50: interface A {",
                path: "file.ts",
                code: "interface A {\na(): void;\nb(): void;\n(): { a: 1 };\nnew (): void;\n}",
                expected: &[],
            },
            Case { name: "invalid 51: type A = {", path: "file.ts", code: "type A = {\na(): void;\nb(): void;\n};", expected: &[] },
            Case {
                name: "invalid 52: declare function f(a: string): string;",
                path: "file.ts",
                code: "declare function f(a: string): string;\nfunction f(a: unknown) { return a; }",
                expected: &[],
            },
            Case {
                name: "invalid 53: export declare function f(a: string): string;",
                path: "file.ts",
                code: "export declare function f(a: string): string;\nexport function f(a: unknown) { return a; }",
                expected: &[],
            },
            Case {
                name: "invalid 54: export default function f(a: string): string;",
                path: "file.ts",
                code: "export default function f(a: string): string;\nexport default function f(a: unknown) { return a; }",
                expected: &[(46, 97, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 55: function f(a: string): string;",
                path: "file.ts",
                code: "function f(a: string): string;\nexport function f(a: unknown) { return a; }",
                expected: &[],
            },
            Case {
                name: "invalid 56: export function f(a: string): string;",
                path: "file.ts",
                code: "export function f(a: string): string;\nconst b = 1;",
                expected: &[(38, 50, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 57: const a = 1;",
                path: "file.ts",
                code: "const a = 1;\nexport = a;\nconst b = 2;",
                expected: &[(25, 37, "Expected blank line before this statement.")],
            },
            Case { name: "invalid 58: const a = 1;", path: "file.ts", code: "const a = 1;\nexport = a;\n\nconst b = 2;", expected: &[] },
            Case {
                name: "invalid 59: import a = require('a');",
                path: "file.ts",
                code: "import a = require('a');\nimport b from 'b';\nimport c = require('c');",
                expected: &[(25, 43, "Expected blank line before this statement."), (44, 68, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 60: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo(); /* a */ /* b */\nbar();\nreturn 1;\n}",
                expected: &[(45, 54, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 61: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo(); /* a\n\n b */\nreturn 1;\n}",
                expected: &[(34, 43, "Expected blank line before this statement.")],
            },
            Case { name: "invalid 62: function f() {", path: "file.ts", code: "function f() {\nfoo();\n/* a */\n\n/* b */\nreturn 1;\n}", expected: &[] },
            Case { name: "invalid 63: function f() {", path: "file.ts", code: "function f() {\nfoo(); // a\n// b\n\nreturn 1;\n}", expected: &[] },
            Case {
                name: "invalid 64: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo() /* a */ ; return 1;\n}",
                expected: &[(31, 40, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 65: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo() /* a\n */ ;\nreturn 1;\n}",
                expected: &[(32, 41, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 66: const a = 1 // c",
                path: "file.ts",
                code: "const a = 1 // c\n;[1].forEach(f)",
                expected: &[(18, 32, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 67: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo()\n;[1].forEach(f)\nreturn 1\n}",
                expected: &[(37, 45, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 68: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo()\n;\nreturn 1\n}",
                expected: &[(23, 31, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 69: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo();\n;\nreturn 1;\n}",
                expected: &[(24, 33, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 70: function f() {",
                path: "file.ts",
                code: "function f() {\n;\nreturn 1;\n}",
                expected: &[(17, 26, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 71: const a = 1;\r",
                path: "file.ts",
                code: "const a = 1;\r\nconst b = {\r\n};",
                expected: &[(14, 25, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 72: const a = 'é';",
                path: "file.ts",
                code: "const a = 'é';\nconst b = { c: 'ü',\n};",
                expected: &[(16, 36, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 73: const a = 'é'; const b = { c: 'ü',",
                path: "file.ts",
                code: "const a = 'é'; const b = { c: 'ü',\n};",
                expected: &[(16, 36, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 74: const a = 1; const b = 2;",
                path: "file.ts",
                code: "const a = 1; const b = 2;",
                expected: &[(15, 27, "Expected blank line before this statement.")],
            },
            Case { name: "invalid 75: const a = 1;", path: "file.ts", code: "const a = 1;\n \t \nconst b = 2;", expected: &[] },
            Case {
                name: "invalid 76: #!/usr/bin/env node",
                path: "file.ts",
                code: "#!/usr/bin/env node\nconst a = 1;\nconst b = 2;",
                expected: &[(33, 45, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 77: class A {",
                path: "file.ts",
                code: "class A {\nstatic {\nfoo();\nif (x) bar();\n}\n}",
                expected: &[(26, 39, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 78: foo();",
                path: "file.ts",
                code: "foo();\n@dec class A {}",
                expected: &[(7, 22, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 79: foo();",
                path: "file.ts",
                code: "foo();\nexport @dec class A {}",
                expected: &[(7, 29, "Expected blank line before this statement.")],
            },
            Case { name: "invalid 80: function f() {", path: "file.ts", code: "function f() {\nfoo();\nabstract class A {}\n}", expected: &[] },
            Case { name: "invalid 81: namespace N {", path: "file.ts", code: "namespace N {\nfoo();\ndeclare const a: {\nb: 1;\n};\nfoo();\n}", expected: &[] },
            Case { name: "invalid 82: namespace N {", path: "file.ts", code: "namespace N {\nfoo();\ndeclare class A {}\nfoo();\n}", expected: &[] },
            Case {
                name: "invalid 83: namespace N {",
                path: "file.ts",
                code: "namespace N {\nfoo();\ndeclare type A = 1;\ndeclare interface B {}\nfoo();\n}",
                expected: &[],
            },
            Case {
                name: "invalid 84: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo();\nlet\nx = 1;\nfoo();\n}",
                expected: &[(22, 25, "Expected blank line before this statement."), (33, 39, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 85: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo();\nconst a = 1, b = function () {\n};\n}",
                expected: &[(22, 52, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 86: function f() {",
                path: "file.ts",
                code: "function f() {\n(() => {})?.();\nfoo();\n}",
                expected: &[(31, 37, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 87: function f() {",
                path: "file.ts",
                code: "function f() {\nvoid (async () => {})();\nfoo();\n}",
                expected: &[(40, 46, "Expected blank line before this statement.")],
            },
            Case { name: "invalid 88: function f() {", path: "file.ts", code: "function f() {\nx?.(() => {});\nfoo();\n}", expected: &[] },
            Case { name: "invalid 89: function f() {", path: "file.ts", code: "function f() {\nfoo() // a\n;[1].forEach(g)\n}", expected: &[] },
            Case {
                name: "invalid 90: function f() {",
                path: "file.ts",
                code: "function f() {\nif (a) {\n} // done\nfoo();\n}",
                expected: &[(34, 40, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 91: function f() {",
                path: "file.ts",
                code: "function f() {\nif (a) {}; // done\nfoo();\n}",
                expected: &[(24, 25, "Expected blank line before this statement.")],
            },
            Case { name: "invalid 92: function f() {", path: "file.ts", code: "function f() {\nx = {\n}\nfoo();\n}", expected: &[] },
            Case {
                name: "invalid 93: function f() {",
                path: "file.ts",
                code: "function f() {\nlabel: x = function () {};\nfoo();\n}",
                expected: &[(42, 48, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 94: class A {",
                path: "file.ts",
                code: "class A {\nm() {\nfoo();\nreturn 1;\n}\n}",
                expected: &[(23, 32, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 95: const o = {",
                path: "file.ts",
                code: "const o = {\nm() {\nfoo();\nreturn 1;\n},\n};",
                expected: &[(25, 34, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 96: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo();\n/** doc */ return 1;\n}",
                expected: &[(33, 42, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 97: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo(); /* a */ return 1;\n}",
                expected: &[(30, 39, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 98: ﻿const a = 1;",
                path: "file.ts",
                code: "﻿const a = 1;\nconst b = 2;",
                expected: &[(16, 28, "Expected blank line before this statement.")],
            },
            Case { name: "invalid 99: const a = 1;", path: "file.ts", code: "const a = 1;\n \nconst b = 2;", expected: &[] },
            Case {
                name: "invalid 100: function f() {",
                path: "file.ts",
                code: "function f() {\ndo {} while (x)\nfoo()\n}",
                expected: &[(31, 36, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 101: function f() {",
                path: "file.ts",
                code: "function f() {\ntry {\n} catch (e) {\n}\nfinally {}\nfoo();\n}",
                expected: &[(48, 54, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 102: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo();\nx = `a\n\nb`;\nreturn 1;\n}",
                expected: &[(34, 43, "Expected blank line before this statement.")],
            },
            Case { name: "invalid 103: function f() {", path: "file.ts", code: "function f() {\nfoo();\n\n\n\nreturn 1;\n}", expected: &[] },
            Case { name: "invalid 104: function f() {", path: "file.ts", code: "function f() {\nfoo();\r\n\r\nreturn 1;\n}", expected: &[] },
            Case { name: "invalid 105: function f() {", path: "file.ts", code: "function f() {\nfoo();\r\rreturn 1;\n}", expected: &[] },
            Case {
                name: "invalid 106: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo();\n// a\n// b\nreturn 1;\n}",
                expected: &[(32, 41, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 107: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo(); // a\nreturn 1; // b\n}",
                expected: &[(27, 36, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 108: export {};",
                path: "file.ts",
                code: "export {};\nfoo();",
                expected: &[(11, 17, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 109: import a from 'a' with { type: 'json' };",
                path: "file.ts",
                code: "import a from 'a' with { type: 'json' };\nimport b from 'b';\nfoo();",
                expected: &[(60, 66, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 110: function f() {",
                path: "file.ts",
                code: "function f() {\nconst a = 1;\nconst b = {\n};\n}",
                expected: &[(28, 39, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 111: function f() {",
                path: "file.ts",
                code: "function f() {\nconst b = {\n};\nconst a = 1;\n}",
                expected: &[(30, 42, "Expected blank line before this statement.")],
            },
            Case { name: "invalid 112: function f() {", path: "file.ts", code: "function f() {\nfoo();\nusing a = b;\n}", expected: &[] },
            Case { name: "invalid 113: function f() {", path: "file.ts", code: "function f() {\nfoo();\nthrow x;\n}", expected: &[] },
            Case {
                name: "invalid 114: function f() {",
                path: "file.ts",
                code: "function f() {\nswitch (a) {}\nfoo();\n}",
                expected: &[(29, 35, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 115: function f() {",
                path: "file.ts",
                code: "function f() {\nfoo()\nreturn 1\n}",
                expected: &[(21, 29, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 116: function f() {",
                path: "file.ts",
                code: "function f() {\nconst a = 1\nreturn a\n}",
                expected: &[(27, 35, "Expected blank line before this statement.")],
            },
            Case { name: "invalid 117: for (;;) {", path: "file.ts", code: "for (;;) {\nfoo();\nbreak;\n}", expected: &[] },
            Case {
                name: "invalid 118: for (;;) {",
                path: "file.ts",
                code: "for (;;) {\nfoo();\nif (x) continue;\n}",
                expected: &[(18, 34, "Expected blank line before this statement.")],
            },
            Case {
                name: "invalid 119: if (a) {",
                path: "file.ts",
                code: "if (a) {\nfoo();\nwhile (b) {}\n}",
                expected: &[(16, 28, "Expected blank line before this statement.")],
            },
        ],
    );
}

/// The fixes, applied the way `fmtkit lint --fix` applies them, against the
/// upstream RuleTester `output` of every invalid case, and against what
/// `oxlint --fix` with the upstream plugin made of every `edges` case.
#[test]
fn fixes() {
    let config = Lint {
        bundled: false,
        rules: BTreeMap::from([("anti-slop/require-readable-spacing".to_owned(), json!("error"))]),
        ignore: Vec::new(),
        ..Default::default()
    };
    let linter = Linter::new(&config).expect("the rule builds");
    let mut failures = String::new();

    for &(code, wanted) in UPSTREAM_FIXES.iter().chain(EDGE_FIXES) {
        let linted = linter.lint("file.ts", Lang::Ts, code, true);
        let fixed = linted.fixed.as_deref().unwrap_or(code);

        if fixed != wanted || !linted.diagnostics.is_empty() {
            let _ = write!(failures, "\n--- {code:?}\nexpected: {wanted:?}\nactual:   {fixed:?}\nleft: {:?}\n", linted.diagnostics);
        }
    }

    assert!(failures.is_empty(), "{failures}");
}

#[test]
fn rejects_options() {
    let factory = super::super::RULES.iter().find(|(name, _)| *name == "anti-slop/require-readable-spacing").map(|(_, factory)| factory).expect("registered");

    assert!(factory(&[]).is_ok());
    assert!(factory(&[json!({})]).is_err());
    assert!(factory(&[json!([{ "blankLine": "always", "prev": "*", "next": "*" }])]).is_err());
}

/// Upstream `output`s, the fix tested by the upstream RuleTester.
const UPSTREAM_FIXES: &[(&str, &str)] = &[
    ("const a = 1;\nconst b = 2;", "const a = 1;\n\nconst b = 2;"),
    ("export const a = 1;\n/** B docs. */\nexport type B = number;", "export const a = 1;\n\n/** B docs. */\nexport type B = number;"),
    ("const a = 1; // trailing\n// leading\nconst b = 2;", "const a = 1; // trailing\n\n// leading\nconst b = 2;"),
    ("const a = 1; const b = 2;", "const a = 1;\n\n const b = 2;"),
    ("import { a } from 'a';\nconst b = a;", "import { a } from 'a';\n\nconst b = a;"),
    ("function f() {\nconst a = 1;\nreturn a;\n}", "function f() {\nconst a = 1;\n\nreturn a;\n}"),
    ("function f() {\nconst a = 1;\nif (a) go();\n}", "function f() {\nconst a = 1;\n\nif (a) go();\n}"),
    ("function f() {\nif (ok) { go(); }\nstop();\n}", "function f() {\nif (ok) { go(); }\n\nstop();\n}"),
    (
        "const f = Effect.gen(function* () {\nconst a = yield* A;\nconst b = yield* B;\nconst dispatch = Effect.fn('dispatch')(function* () {\nyield* a;\n});\nreturn dispatch;\n});",
        "const f = Effect.gen(function* () {\nconst a = yield* A;\nconst b = yield* B;\n\nconst dispatch = Effect.fn('dispatch')(function* () {\nyield* a;\n});\n\nreturn dispatch;\n});",
    ),
    ("export interface A {}\nexport class B {}", "export interface A {}\n\nexport class B {}"),
    ("const a = 1\n;[1].forEach(f)", "const a = 1\n\n;[1].forEach(f)"),
    ("function f() {\nfoo();\nwhile (ok) go();\n}", "function f() {\nfoo();\n\nwhile (ok) go();\n}"),
];

/// The `edges` cases with findings, fixed by `oxlint --fix` with the upstream plugin.
const EDGE_FIXES: &[(&str, &str)] = &[
    ("function f() {\nfoo();\n{\nbar();\n}\nbaz();\n}", "function f() {\nfoo();\n{\nbar();\n}\n\nbaz();\n}"),
    (
        "function f() {\nfoo();\nswitch (x) {\ncase 1: {\n}\ncase 2:\nbreak;\n}\nbar();\n}",
        "function f() {\nfoo();\n\nswitch (x) {\ncase 1: {\n}\n\ncase 2:\nbreak;\n}\n\nbar();\n}",
    ),
    (
        "switch (x) {\ncase 1: { a(); }\ncase 2: b();\ncase 3: if (y) { c(); }\ndefault:\nd();\n}",
        "switch (x) {\ncase 1: { a(); }\n\ncase 2: b();\ncase 3: if (y) { c(); }\n\ndefault:\nd();\n}",
    ),
    ("switch (x) {\ncase 1:\na();\nreturn;\n}", "switch (x) {\ncase 1:\na();\n\nreturn;\n}"),
    ("\"use strict\";\nconst a = 1;", "\"use strict\";\n\nconst a = 1;"),
    ("function f() {\n\"use strict\";\nreturn 1;\n}", "function f() {\n\"use strict\";\n\nreturn 1;\n}"),
    ("foo();\nlabel: for (;;) {}", "foo();\n\nlabel: for (;;) {}"),
    ("function f() {\nfoo();\nouter: for (;;) {}\n}", "function f() {\nfoo();\n\nouter: for (;;) {}\n}"),
    ("function f() {\nfoo();\nlabel: {\n}\nbar();\n}", "function f() {\nfoo();\nlabel: {\n}\n\nbar();\n}"),
    ("function f() {\nconst a = () => {\n};\nfoo();\n}", "function f() {\nconst a = () => {\n};\n\nfoo();\n}"),
    ("function f() {\nconst g = () => {};\nfoo();\n}", "function f() {\nconst g = () => {};\n\nfoo();\n}"),
    ("function f() {\n(function () {})();\nfoo();\n}", "function f() {\n(function () {})();\n\nfoo();\n}"),
    ("function f() {\n!function () {}();\nfoo();\n}", "function f() {\n!function () {}();\n\nfoo();\n}"),
    ("function f() {\n(0, () => {})();\nfoo();\n}", "function f() {\n(0, () => {})();\n\nfoo();\n}"),
    ("function f() {\nx = function () {};\nfoo();\n}", "function f() {\nx = function () {};\n\nfoo();\n}"),
    ("function f() {\nnew function () {};\nfoo();\n}", "function f() {\nnew function () {};\n\nfoo();\n}"),
    ("function f() {\nx = a ? b : () => {};\nfoo();\n}", "function f() {\nx = a ? b : () => {};\n\nfoo();\n}"),
    ("function f() {\ntry {} catch {}\nfoo();\n}", "function f() {\ntry {} catch {}\n\nfoo();\n}"),
    ("function f() {\ntry {} finally {}\nfoo();\n}", "function f() {\ntry {} finally {}\n\nfoo();\n}"),
    ("function f() {\ndo {\n} while (x);\nfoo();\n}", "function f() {\ndo {\n} while (x);\n\nfoo();\n}"),
    ("function f() {\nfoo();\ndo foo(); while (x);\nbar();\n}", "function f() {\nfoo();\n\ndo foo(); while (x);\nbar();\n}"),
    ("function f() {\nwhile (a) {}\nfoo();\n}", "function f() {\nwhile (a) {}\n\nfoo();\n}"),
    ("function f() {\nfoo();\nreturn () => {\n};\n}", "function f() {\nfoo();\n\nreturn () => {\n};\n}"),
    ("function f() {\nreturn function () {};\nfoo();\n}", "function f() {\nreturn function () {};\n\nfoo();\n}"),
    (
        "function f() {\nlet a = 1, b = 2;\nlet c = {\n};\nvar d = 1;\nvar e = [\n];\nfoo();\n}",
        "function f() {\nlet a = 1, b = 2;\n\nlet c = {\n};\n\nvar d = 1;\n\nvar e = [\n];\n\nfoo();\n}",
    ),
    (
        "async function f() {\nfoo();\nawait using a = b(\n);\nusing c = d;\nfoo();\n}",
        "async function f() {\nfoo();\n\nawait using a = b(\n);\n\nusing c = d;\nfoo();\n}",
    ),
    (
        "async function f() {\nfoo();\nfor (const a of b) go();\nfor (const a in b) go();\nfor await (const a of b) go();\n}",
        "async function f() {\nfoo();\n\nfor (const a of b) go();\n\nfor (const a in b) go();\n\nfor await (const a of b) go();\n}",
    ),
    ("function f() {\nclass A {}\nfoo();\n}", "function f() {\nclass A {}\n\nfoo();\n}"),
    ("function f() {\nfoo();\nclass A {}\n}", "function f() {\nfoo();\n\nclass A {}\n}"),
    ("function f() {\nfoo();\nfunction g() {}\n}", "function f() {\nfoo();\n\nfunction g() {}\n}"),
    ("function f() {\nfunction g() {}\nfoo();\n}", "function f() {\nfunction g() {}\n\nfoo();\n}"),
    ("function f() {\nfoo();\ntype A = string;\ninterface B {}\nenum C {}\n}", "function f() {\nfoo();\n\ntype A = string;\n\ninterface B {}\n\nenum C {}\n}"),
    ("declare namespace N {\nclass A {}\nconst b: 1;\n}", "declare namespace N {\nclass A {}\n\nconst b: 1;\n}"),
    (
        "namespace N {\nexport function f(a: string): string;\nexport function f(a: number) { return a; }\nconst b = 1;\n}",
        "namespace N {\nexport function f(a: string): string;\nexport function f(a: number) { return a; }\n\nconst b = 1;\n}",
    ),
    (
        "declare module 'x' {\nimport a from 'a';\nimport b from 'b';\nconst c: 1;\n}",
        "declare module 'x' {\nimport a from 'a';\nimport b from 'b';\n\nconst c: 1;\n}",
    ),
    (
        "export default function f(a: string): string;\nexport default function f(a: unknown) { return a; }",
        "export default function f(a: string): string;\n\nexport default function f(a: unknown) { return a; }",
    ),
    ("export function f(a: string): string;\nconst b = 1;", "export function f(a: string): string;\n\nconst b = 1;"),
    ("const a = 1;\nexport = a;\nconst b = 2;", "const a = 1;\n\nexport = a;\nconst b = 2;"),
    ("import a = require('a');\nimport b from 'b';\nimport c = require('c');", "import a = require('a');\n\nimport b from 'b';\n\nimport c = require('c');"),
    ("function f() {\nfoo(); /* a */ /* b */\nbar();\nreturn 1;\n}", "function f() {\nfoo(); /* a */ /* b */\nbar();\n\nreturn 1;\n}"),
    ("function f() {\nfoo(); /* a\n\n b */\nreturn 1;\n}", "function f() {\nfoo(); /* a\n\n b */\n\nreturn 1;\n}"),
    ("function f() {\nfoo() /* a */ ; return 1;\n}", "function f() {\nfoo() /* a */ ;\n\n return 1;\n}"),
    ("function f() {\nfoo() /* a\n */ ;\nreturn 1;\n}", "function f() {\nfoo() /* a\n */ ;\n\nreturn 1;\n}"),
    ("const a = 1 // c\n;[1].forEach(f)", "const a = 1 // c\n\n;[1].forEach(f)"),
    ("function f() {\nfoo()\n;[1].forEach(f)\nreturn 1\n}", "function f() {\nfoo()\n;[1].forEach(f)\n\nreturn 1\n}"),
    ("function f() {\nfoo()\n;\nreturn 1\n}", "function f() {\nfoo()\n;\n\nreturn 1\n}"),
    ("function f() {\nfoo();\n;\nreturn 1;\n}", "function f() {\nfoo();\n;\n\nreturn 1;\n}"),
    ("function f() {\n;\nreturn 1;\n}", "function f() {\n;\n\nreturn 1;\n}"),
    ("const a = 1;\r\nconst b = {\r\n};", "const a = 1;\n\r\nconst b = {\r\n};"),
    ("const a = '\u{00e9}';\nconst b = { c: '\u{00fc}',\n};", "const a = '\u{00e9}';\n\nconst b = { c: '\u{00fc}',\n};"),
    ("const a = '\u{00e9}'; const b = { c: '\u{00fc}',\n};", "const a = '\u{00e9}';\n\n const b = { c: '\u{00fc}',\n};"),
    ("const a = 1;\u{2028}const b = 2;", "const a = 1;\n\u{2028}const b = 2;"),
    ("#!/usr/bin/env node\nconst a = 1;\nconst b = 2;", "#!/usr/bin/env node\nconst a = 1;\n\nconst b = 2;"),
    ("class A {\nstatic {\nfoo();\nif (x) bar();\n}\n}", "class A {\nstatic {\nfoo();\n\nif (x) bar();\n}\n}"),
    ("foo();\n@dec class A {}", "foo();\n\n@dec class A {}"),
    ("foo();\nexport @dec class A {}", "foo();\n\nexport @dec class A {}"),
    ("function f() {\nfoo();\nlet\nx = 1;\nfoo();\n}", "function f() {\nfoo();\n\nlet\nx = 1;\n\nfoo();\n}"),
    ("function f() {\nfoo();\nconst a = 1, b = function () {\n};\n}", "function f() {\nfoo();\n\nconst a = 1, b = function () {\n};\n}"),
    ("function f() {\n(() => {})?.();\nfoo();\n}", "function f() {\n(() => {})?.();\n\nfoo();\n}"),
    ("function f() {\nvoid (async () => {})();\nfoo();\n}", "function f() {\nvoid (async () => {})();\n\nfoo();\n}"),
    ("function f() {\nif (a) {\n} // done\nfoo();\n}", "function f() {\nif (a) {\n} // done\n\nfoo();\n}"),
    ("function f() {\nif (a) {}; // done\nfoo();\n}", "function f() {\nif (a) {}\n\n; // done\nfoo();\n}"),
    ("function f() {\nlabel: x = function () {};\nfoo();\n}", "function f() {\nlabel: x = function () {};\n\nfoo();\n}"),
    ("class A {\nm() {\nfoo();\nreturn 1;\n}\n}", "class A {\nm() {\nfoo();\n\nreturn 1;\n}\n}"),
    ("const o = {\nm() {\nfoo();\nreturn 1;\n},\n};", "const o = {\nm() {\nfoo();\n\nreturn 1;\n},\n};"),
    ("function f() {\nfoo();\n/** doc */ return 1;\n}", "function f() {\nfoo();\n\n/** doc */ return 1;\n}"),
    ("function f() {\nfoo(); /* a */ return 1;\n}", "function f() {\nfoo(); /* a */\n\n return 1;\n}"),
    ("\u{feff}const a = 1;\nconst b = 2;", "\u{feff}const a = 1;\n\nconst b = 2;"),
    ("function f() {\ndo {} while (x)\nfoo()\n}", "function f() {\ndo {} while (x)\n\nfoo()\n}"),
    ("function f() {\ntry {\n} catch (e) {\n}\nfinally {}\nfoo();\n}", "function f() {\ntry {\n} catch (e) {\n}\nfinally {}\n\nfoo();\n}"),
    ("function f() {\nfoo();\nx = `a\n\nb`;\nreturn 1;\n}", "function f() {\nfoo();\nx = `a\n\nb`;\n\nreturn 1;\n}"),
    ("function f() {\nfoo();\n// a\n// b\nreturn 1;\n}", "function f() {\nfoo();\n\n// a\n// b\nreturn 1;\n}"),
    ("function f() {\nfoo(); // a\nreturn 1; // b\n}", "function f() {\nfoo(); // a\n\nreturn 1; // b\n}"),
    ("export {};\nfoo();", "export {};\n\nfoo();"),
    ("import a from 'a' with { type: 'json' };\nimport b from 'b';\nfoo();", "import a from 'a' with { type: 'json' };\nimport b from 'b';\n\nfoo();"),
    ("function f() {\nconst a = 1;\nconst b = {\n};\n}", "function f() {\nconst a = 1;\n\nconst b = {\n};\n}"),
    ("function f() {\nconst b = {\n};\nconst a = 1;\n}", "function f() {\nconst b = {\n};\n\nconst a = 1;\n}"),
    ("function f() {\nswitch (a) {}\nfoo();\n}", "function f() {\nswitch (a) {}\n\nfoo();\n}"),
    ("function f() {\nfoo()\nreturn 1\n}", "function f() {\nfoo()\n\nreturn 1\n}"),
    ("function f() {\nconst a = 1\nreturn a\n}", "function f() {\nconst a = 1\n\nreturn a\n}"),
    ("for (;;) {\nfoo();\nif (x) continue;\n}", "for (;;) {\nfoo();\n\nif (x) continue;\n}"),
    ("if (a) {\nfoo();\nwhile (b) {}\n}", "if (a) {\nfoo();\n\nwhile (b) {}\n}"),
];
