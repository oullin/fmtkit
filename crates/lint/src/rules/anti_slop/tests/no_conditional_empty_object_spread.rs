//! `anti-slop/no-conditional-empty-object-spread`: expectations recorded from the v1 plugin under oxlint 1.86.

use super::{Case, check};

/// tests/no-conditional-empty-object-spread.test.ts
#[test]
fn v1() {
    check(
        "anti-slop/no-conditional-empty-object-spread",
        "[]",
        &[
            Case {
                name: "unconditional spread",
                path: "case.ts",
                code: "declare const base: { id: string };\n\nexport const record = { ...base, active: true };\n",
                expected: &[],
            },
            Case {
                name: "conditional empty spread",
                path: "case.ts",
                code: "declare const flag: boolean;\ndeclare const extra: { id: string };\n\nexport const record = { ...(flag ? extra : {}) };\n",
                expected: &[(91, 113, "This conditional spread hides property omission behind an empty object. Build the object in separate statements and add the property only when present.")],
            },
            Case {
                name: "conditional empty spread on the left",
                path: "case.ts",
                code: "declare const flag: boolean;\ndeclare const extra: { id: string };\n\nexport const record = { ...(flag ? {} : extra) };\n",
                expected: &[(91, 113, "This conditional spread hides property omission behind an empty object. Build the object in separate statements and add the property only when present.")],
            },
        ],
    );
}

#[test]
fn edges() {
    check(
        "anti-slop/no-conditional-empty-object-spread",
        "[]",
        &[
            Case {
                name: "unparenthesized conditional",
                path: "case.ts",
                code: "declare const flag: boolean;\nexport const a = { ...flag ? { id: 1 } : {} };\n",
                expected: &[(48, 72, "This conditional spread hides property omission behind an empty object. Build the object in separate statements and add the property only when present.")],
            },
            Case {
                name: "parenthesized empty branch",
                path: "case.ts",
                code: "declare const flag: boolean;\nexport const a = { ...(flag ? ({}) : { id: 1 }) };\n",
                expected: &[(48, 76, "This conditional spread hides property omission behind an empty object. Build the object in separate statements and add the property only when present.")],
            },
            Case {
                name: "non-empty branches",
                path: "case.ts",
                code: "declare const flag: boolean;\nexport const a = { ...(flag ? { id: 1 } : { id: 2 }) };\n",
                expected: &[],
            },
            Case {
                name: "array spread is not an object spread",
                path: "case.ts",
                code: "declare const flag: boolean;\nexport const a = [...(flag ? [] : [1])];\nexport const b = Object.assign({}, ...[flag ? {} : { id: 1 }]);\n",
                expected: &[],
            },
            Case {
                name: "call argument spread",
                path: "case.ts",
                code: "declare const flag: boolean;\ndeclare function f(...a: object[]): void;\nf(...(flag ? {} : { id: 1 }) as never);\n",
                expected: &[],
            },
            Case {
                name: "logical spread is fine",
                path: "case.ts",
                code: "declare const flag: boolean;\nexport const a = { ...(flag && { id: 1 }) };\n",
                expected: &[],
            },
            Case {
                name: "nested object spread",
                path: "case.ts",
                code: "declare const flag: boolean;\nexport const a = { b: { ...(flag ? {} : { id: 1 }) } };\n",
                expected: &[(53, 79, "This conditional spread hides property omission behind an empty object. Build the object in separate statements and add the property only when present.")],
            },
            Case {
                name: "object pattern rest is not a spread element",
                path: "case.ts",
                code: "declare const o: { a: number };\nexport const { ...rest } = o;\n",
                expected: &[],
            },
        ],
    );
}
