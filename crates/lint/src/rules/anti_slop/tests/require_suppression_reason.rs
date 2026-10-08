//! `anti-slop/require-suppression-reason`: expectations recorded from the v1 plugin under oxlint 1.86.

use super::{Case, check};

/// tests/require-suppression-reason.test.ts
#[test]
fn v1() {
    check(
        "anti-slop/require-suppression-reason",
        "[]",
        &[
            Case {
                name: "ordinary comment",
                path: "case.ts",
                code: "// The pool size follows the host, not the input.\nexport const size = 1;\n",
                expected: &[],
            },
            Case {
                name: "named rule with a reason",
                path: "case.ts",
                code: "// oxlint-disable-next-line anti-slop/no-unknown-parameters -- this is the boundary parser, and it must admit arbitrary payloads to reject them.\nexport function parse(payload: unknown): string {\n\treturn String(payload);\n}\n",
                expected: &[],
            },
            Case {
                name: "blanket disable",
                path: "case.ts",
                code: "// oxlint-disable\nexport const size = 1;\n",
                expected: &[(
                    0,
                    17,
                    "This suppression names no rule, so it silences every future diagnostic on its target. Name the rules it is meant to silence.",
                )],
            },
            Case {
                name: "named rule without a reason",
                path: "case.ts",
                code: "// oxlint-disable-next-line anti-slop/no-unknown-parameters\nexport function parse(payload: unknown): string {\n\treturn String(payload);\n}\n",
                expected: &[(0, 59, "This suppression has no justification. Append `-- <reason>` explaining the invariant that makes it safe.")],
            },
            Case {
                name: "placeholder reason",
                path: "case.ts",
                code: "// oxlint-disable-next-line anti-slop/no-unknown-parameters -- needed\nexport function parse(payload: unknown): string {\n\treturn String(payload);\n}\n",
                expected: &[(0, 69, "This suppression's justification is too short to be one. State the invariant that makes silencing the rule correct.")],
            },
            Case {
                name: "directive for another linter",
                path: "case.ts",
                code: "// eslint-disable-next-line no-console -- a real reason, but for a linter this repo does not run.\nexport const size = 1;\n",
                expected: &[(0, 97, "This directive targets another linter. Write `oxlint-disable-next-line <rule> -- <reason>`, or delete it.")],
            },
        ],
    );
}

#[test]
fn edges() {
    check(
        "anti-slop/require-suppression-reason",
        "[]",
        &[
            Case {
                name: "directive forms",
                path: "case.ts",
                code: "// oxlint-disable-next-line\nexport const a = 1;\n// oxlint-disable-line\nexport const b = 1; // oxlint-disable-line no-console\n/* oxlint-disable */\n/* oxlint-disable no-console -- this is long enough to count */\n/*oxlint-disable no-console*/\n// oxlint-enable\n// oxlint-disabled\n// oxlint-disable -- reason without rules here\n// oxlint-disable no-console --short\n// oxlint-disable no-console --  exactly sixteen!\n// oxlint-disable no-console -- fifteen chars!!\n",
                expected: &[
                    (0, 27, "This suppression has no justification. Append `-- <reason>` explaining the invariant that makes it safe."),
                    (91, 124, "This suppression has no justification. Append `-- <reason>` explaining the invariant that makes it safe."),
                    (276, 322, "This suppression names no rule, so it silences every future diagnostic on its target. Name the rules it is meant to silence."),
                ],
            },
            Case {
                name: "foreign directives",
                path: "case.ts",
                code: "// prettier-ignore\nexport const a = 1;\n/* biome-ignore lint: x */\n// tslint:disable\n// tslint-disable\n// eslint-disable\n// eslint-enable\n// eslint-disabled\n//eslint-ignore\n",
                expected: &[
                    (0, 18, "This directive targets another linter. Write `oxlint-disable-next-line <rule> -- <reason>`, or delete it."),
                    (39, 65, "This directive targets another linter. Write `oxlint-disable-next-line <rule> -- <reason>`, or delete it."),
                    (84, 101, "This directive targets another linter. Write `oxlint-disable-next-line <rule> -- <reason>`, or delete it."),
                    (102, 119, "This directive targets another linter. Write `oxlint-disable-next-line <rule> -- <reason>`, or delete it."),
                    (156, 171, "This directive targets another linter. Write `oxlint-disable-next-line <rule> -- <reason>`, or delete it."),
                ],
            },
            Case { name: "leading text is not a directive", path: "case.ts", code: "// see oxlint-disable docs\n// text eslint-disable\n", expected: &[] },
            Case {
                name: "unicode reason length counts utf-16 units",
                path: "case.ts",
                code: "// oxlint-disable no-console -- 😀😀😀😀😀😀😀😀\n// oxlint-disable no-console -- ééééééééééééééé\nexport const a = 1;\n",
                expected: &[(65, 127, "This suppression's justification is too short to be one. State the invariant that makes silencing the rule correct.")],
            },
            Case {
                name: "multi-line block directive",
                path: "case.ts",
                code: "/*\n * oxlint-disable no-console -- a long enough justification\n */\n/*\n\toxlint-disable\n*/\nexport const a = 1;\n",
                expected: &[],
            },
        ],
    );
}
