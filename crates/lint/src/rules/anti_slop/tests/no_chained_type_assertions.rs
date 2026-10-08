//! `anti-slop/no-chained-type-assertions`: expectations recorded from the v1 plugin under oxlint 1.86.

use super::{Case, check};

/// tests/no-chained-type-assertions.test.ts
#[test]
fn v1() {
    check(
        "anti-slop/no-chained-type-assertions",
        "[]",
        &[
            Case {
                name: "no assertion",
                path: "case.ts",
                code: "export const total: number = 1;\n",
                expected: &[],
            },
            Case {
                name: "const assertion chain",
                path: "case.ts",
                code: "export const modes = [\"read\", \"write\"] as const;\n",
                expected: &[],
            },
            Case {
                name: "double assertion through unknown",
                path: "case.ts",
                code: "declare const raw: string;\n\n// SAFETY: fixture.\nexport const total = raw as unknown as number;\n",
                expected: &[(69, 93, "This assertion chain discards type evidence. Keep the original precise type, or parse untrusted input at its boundary before narrowing it.")],
            },
            Case {
                name: "parenthesized chain",
                path: "case.ts",
                code: "declare const raw: string;\n\n// SAFETY: fixture.\nexport const total = (raw as unknown) as number;\n",
                expected: &[(69, 95, "This assertion chain discards type evidence. Keep the original precise type, or parse untrusted input at its boundary before narrowing it.")],
            },
        ],
    );
}

#[test]
fn edges() {
    check(
        "anti-slop/no-chained-type-assertions",
        "[]",
        &[
            Case {
                name: "triple chain reports once",
                path: "case.ts",
                code: "declare const raw: string;\nexport const a = raw as unknown as object as number;\n",
                expected: &[(44, 78, "This assertion chain discards type evidence. Keep the original precise type, or parse untrusted input at its boundary before narrowing it.")],
            },
            Case {
                name: "angle bracket chain",
                path: "case.ts",
                code: "declare const raw: string;\nexport const a = <number>(<unknown>raw);\n",
                expected: &[(44, 66, "This assertion chain discards type evidence. Keep the original precise type, or parse untrusted input at its boundary before narrowing it.")],
            },
            Case {
                name: "mixed chain",
                path: "case.ts",
                code: "declare const raw: string;\nexport const a = (<unknown>raw) as number;\nexport const b = <number>(raw as unknown);\n",
                expected: &[(44, 68, "This assertion chain discards type evidence. Keep the original precise type, or parse untrusted input at its boundary before narrowing it."), (87, 111, "This assertion chain discards type evidence. Keep the original precise type, or parse untrusted input at its boundary before narrowing it.")],
            },
            Case {
                name: "const of const",
                path: "case.ts",
                code: "export const a = ([1] as const) as const;\n",
                expected: &[],
            },
            Case {
                name: "const then type",
                path: "case.ts",
                code: "export const a = ([1] as const) as readonly number[];\n",
                expected: &[(17, 52, "This assertion chain discards type evidence. Keep the original precise type, or parse untrusted input at its boundary before narrowing it.")],
            },
            Case {
                name: "satisfies breaks the chain",
                path: "case.ts",
                code: "declare const raw: string;\nexport const a = (raw as unknown satisfies unknown) as number;\n",
                expected: &[],
            },
            Case {
                name: "non-null breaks the chain",
                path: "case.ts",
                code: "declare const raw: string;\nexport const a = (raw as unknown)! as number;\n",
                expected: &[],
            },
            Case {
                name: "deep parentheses",
                path: "case.ts",
                code: "declare const raw: string;\nexport const a = (((raw as unknown))) as number;\n",
                expected: &[(44, 74, "This assertion chain discards type evidence. Keep the original precise type, or parse untrusted input at its boundary before narrowing it.")],
            },
            Case {
                name: "separate assertions",
                path: "case.ts",
                code: "declare const raw: string;\nexport const a = [raw as unknown, raw as string];\n",
                expected: &[],
            },
            Case {
                name: "chain inside a call argument",
                path: "case.ts",
                code: "declare const raw: string;\nexport const a = String(raw as unknown as number);\n",
                expected: &[(51, 75, "This assertion chain discards type evidence. Keep the original precise type, or parse untrusted input at its boundary before narrowing it.")],
            },
        ],
    );
}
