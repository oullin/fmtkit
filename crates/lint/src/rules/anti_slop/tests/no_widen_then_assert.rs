//! `anti-slop/no-widen-then-assert`: expectations recorded from the v1 plugin under oxlint 1.86.

use super::{Case, check};

/// tests/no-widen-then-assert.test.ts
#[test]
fn v1() {
    check(
        "anti-slop/no-widen-then-assert",
        "[]",
        &[
            Case {
                name: "precise type survives",
                path: "case.ts",
                code: "export function read(): string {\n\tconst owner = { id: \"a\" };\n\n\treturn owner.id;\n}\n",
                expected: &[],
            },
            Case {
                name: "named contract, no assertion",
                path: "case.ts",
                code: "type Owner = { readonly id: string };\n\nexport function read(): Owner {\n\tconst owner: Owner = { id: \"a\" };\n\n\treturn owner;\n}\n",
                expected: &[],
            },
            Case {
                name: "widened to unknown then asserted back",
                path: "case.ts",
                code: "type Owner = { readonly id: string };\n\nexport function read(): Owner {\n\tconst owner: unknown = { id: \"a\" };\n\n\treturn owner as Owner;\n}\n",
                expected: &[(117, 131, "Binding \"owner\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.")],
            },
            Case {
                name: "widened at module scope",
                path: "case.ts",
                code: "type Owner = { readonly id: string };\n\nconst owner: unknown = { id: \"a\" };\n\nexport const found = owner as Owner;\n",
                expected: &[(97, 111, "Binding \"owner\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.")],
            },
        ],
    );
}

/// tests/no-widen-then-assert.test.ts: open-record and object widening belong to the neighbours.
#[test]
fn v1_neighbours() {
    check(
        "anti-slop/no-widen-then-assert",
        "[]",
        &[
            Case {
                name: "open record",
                path: "case.ts",
                code: "type Owner = { readonly id: string };\n\nexport function read(): Owner {\n\tconst owner: Record<string, unknown> = { id: \"a\" };\n\n\treturn owner as Owner;\n}\n",
                expected: &[],
            },
            Case {
                name: "object",
                path: "case.ts",
                code: "type Owner = { readonly id: string };\n\nexport function read(): Owner {\n\tconst owner: object = { id: \"a\" };\n\n\treturn owner as Owner;\n}\n",
                expected: &[],
            },
        ],
    );
}

#[test]
fn broad_kinds() {
    check(
        "anti-slop/no-widen-then-assert",
        "[]",
        &[
            Case {
                name: "any and angle brackets",
                path: "case.ts",
                code: "const a: any = 1;\nexport const b = <number>a;\nexport const c = (a) as string;\n",
                expected: &[(35, 44, "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once."), (63, 76, "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.")],
            },
            Case {
                name: "widened through an initializer assertion",
                path: "case.ts",
                code: "const a = { id: 1 } as unknown;\nexport const b = a as { id: number };\nconst c = ({ id: 1 }) as object;\nexport const d = c as { id: number };\nconst e = [1] as Record<string, unknown>;\nexport const f = e as Record<string, number>;\n",
                expected: &[(49, 68, "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once."), (120, 139, "Binding \"c\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once."), (200, 227, "Binding \"e\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.")],
            },
            Case {
                name: "object kind needs a definitely object assertion",
                path: "case.ts",
                code: "const a: object = { id: 1 };\nexport const b = a as string;\nexport const c = a as number[];\nexport const d = a as () => void;\nexport const e = a as new () => void;\nexport const f = a as { [K in string]: 1 };\nexport const g = a as [number];\nexport const h = a as {};\nexport const i = a as { id: number } & { b: 1 };\nexport const j = a as readonly number[];\nexport const k = a as object;\nexport const l = a as unknown;\n",
                expected: &[(76, 89, "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once."), (108, 123, "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once."), (142, 161, "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once."), (180, 205, "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once."), (224, 237, "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once."), (282, 312, "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once."), (331, 353, "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.")],
            },
            Case {
                name: "record kind needs a narrower record",
                path: "case.ts",
                code: "const a: Record<string, unknown> = { id: 1 };\nexport const b = a as { id: number };\nexport const c = a as Record<string, number>;\nexport const d = a as Readonly<Record<string, number>>;\nexport const e = a as { [key: string]: number };\nexport const f = a as Record<string, any>;\nexport const g = a as Map<string, number>;\nexport const h = a as Record<string>;\n",
                expected: &[(63, 82, "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once."), (101, 128, "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once."), (147, 184, "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.")],
            },
            Case {
                name: "record forms",
                path: "case.ts",
                code: "const a: Readonly<Record<PropertyKey, any>> = {};\nexport const b = a as Record<string, number>;\nconst c: { [key: string | number]: unknown } = {};\nexport const d = a as Record<string, number>;\nconst e: { [key: string]: unknown; b: 1 } = { b: 1 };\nexport const f = e as Record<string, number>;\nconst g: Record<`x${string}`, unknown> = {};\nexport const h = g as Record<string, number>;\n",
                expected: &[(67, 94, "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once."), (164, 191, "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.")],
            },
            Case {
                name: "same syntax as the evidence",
                path: "case.ts",
                code: "type Box = { id: number };\nconst a: object = { id: 1 } as Box;\nexport const b = a as Box;\nconst c: Record<string, unknown> = { id: 1 } as Box;\nexport const d = c as  Box;\nconst e: object = 1 as (Box);\nexport const f = e as Box;\n",
                expected: &[(80, 88, "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once."), (160, 169, "Binding \"c\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once."), (218, 226, "Binding \"e\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.")],
            },
        ],
    );
}

#[test]
fn evidence() {
    check(
        "anti-slop/no-widen-then-assert",
        "[]",
        &[
            Case {
                name: "evidence kinds",
                path: "case.ts",
                code: "const a: unknown = \"x\";\nexport const b = a as string;\nconst c: unknown = `x`;\nexport const d = c as string;\nconst e: unknown = new Map();\nexport const f = e as Map<string, number>;\nconst g: unknown = () => 1;\nexport const h = g as () => number;\nconst i: unknown = read();\nexport const j = i as number;\ndeclare function read(): number;\nconst k: unknown = -1;\nexport const l = k as number;\n",
                expected: &[(41, 52, "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once."), (95, 106, "Binding \"c\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once."), (155, 179, "Binding \"e\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once."), (226, 243, "Binding \"g\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.")],
            },
            Case {
                name: "evidence through bindings",
                path: "case.ts",
                code: "const base = { id: 1 };\nconst a: unknown = base;\nexport const b = a as { id: number };\nlet loose = { id: 1 };\nconst c: unknown = loose;\nexport const d = c as { id: number };\nconst typed: { id: number } = { id: 1 };\nconst e: unknown = typed;\nexport const f = e as { id: number };\nconst broad: unknown = { id: 1 };\nconst g: unknown = broad;\nexport const h = g as { id: number };\n",
                expected: &[(66, 85, "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once."), (258, 277, "Binding \"e\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.")],
            },
            Case {
                name: "parameters as evidence",
                path: "case.ts",
                code: "export function f(x: { id: number }, y: unknown, ...z: number[]): void {\n\tconst a: unknown = x;\n\tconst b: unknown = y;\n\tconst c: unknown = z;\n\tconsole.log(a as { id: number }, b as number, c as number[]);\n}\n",
                expected: &[(155, 174, "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.")],
            },
            Case {
                name: "evidence across function boundaries",
                path: "case.ts",
                code: "const outer = { id: 1 };\nexport function f(): void {\n\tconst a: unknown = outer;\n\tconsole.log(a as { id: number });\n}\nconst b: unknown = { id: 1 };\nexport function g(): unknown {\n\treturn b as { id: number };\n}\nexport const h = (): unknown => b as { id: number };\n",
                expected: &[],
            },
            Case {
                name: "assertion before declaration end",
                path: "case.ts",
                code: "const a: unknown = { id: 1 };\nconst b: unknown = (b as never);\nexport { a };\n",
                expected: &[],
            },
            Case {
                name: "binding is reassigned or not const",
                path: "case.ts",
                code: "let a: unknown = { id: 1 };\nexport const b = a as { id: number };\nvar c: unknown = { id: 1 };\nexport const d = c as { id: number };\n",
                expected: &[],
            },
            Case {
                name: "destructured and missing init",
                path: "case.ts",
                code: "const { a }: { a: unknown } = { a: 1 };\nexport const b = a as number;\ndeclare const c: unknown;\nexport const d = c as number;\n",
                expected: &[],
            },
            Case {
                name: "catch parameter",
                path: "case.ts",
                code: "try {\n\tvoid 0;\n} catch (error: unknown) {\n\tconst a: unknown = error;\n\tconsole.log(a as Error);\n}\n",
                expected: &[],
            },
            Case {
                name: "broad assertion is not narrowing",
                path: "case.ts",
                code: "const a: unknown = { id: 1 };\nexport const b = a as any;\nexport const c = a as Record<string, unknown>;\n",
                expected: &[],
            },
            Case {
                name: "self referential evidence",
                path: "case.ts",
                code: "const a: unknown = a;\nexport const b = a as number;\n",
                expected: &[],
            },
        ],
    );
}
