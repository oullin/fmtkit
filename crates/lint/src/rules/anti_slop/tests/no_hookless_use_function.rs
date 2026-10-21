//! `anti-slop/no-hookless-use-function`: adapted from t3code's `t3code/no-hookless-use-function`.

use super::{Case, check};

/// The rule's behaviour, written for fmtkit rather than recorded from t3code's plugin.
#[test]
fn adapted() {
    check(
        "anti-slop/no-hookless-use-function",
        "[]",
        &[
            Case {
                name: "hookless declaration in tsx",
                path: "case.tsx",
                code: "export function useTitle(name: string): string {\n\treturn name.toUpperCase();\n}\n",
                expected: &[(
                    16,
                    24,
                    "`useTitle` calls no hooks, so the React Compiler skips it and whatever it returns is rebuilt on every render. Rename it to a plain function, or call the hook it depends on inside it.",
                )],
            },
            Case {
                name: "calls useState",
                path: "case.tsx",
                code: "import { useState } from \"react\";\n\nexport function useCount(): number {\n\tconst [n] = useState(0);\n\n\treturn n;\n}\n",
                expected: &[],
            },
            Case {
                name: "calls React.useState",
                path: "case.tsx",
                code: "export function useCount(): number {\n\treturn React.useState(0)[0];\n}\n",
                expected: &[],
            },
            Case { name: "calls use", path: "case.tsx", code: "export function useValue(p: Promise<number>): number {\n\treturn use(p);\n}\n", expected: &[] },
            Case {
                name: "hook only in a callback",
                path: "case.tsx",
                code: "export const useLater = () => {\n\treturn () => useContext(Theme);\n};\n",
                expected: &[(
                    13,
                    21,
                    "`useLater` calls no hooks, so the React Compiler skips it and whatever it returns is rebuilt on every render. Rename it to a plain function, or call the hook it depends on inside it.",
                )],
            },
            Case {
                name: "function expression in a file importing react",
                path: "case.ts",
                code: "import { memo } from \"react\";\n\nexport const useTitle = function (name: string): string {\n\treturn name;\n};\n\nexport const Title = memo(() => null);\n",
                expected: &[(
                    44,
                    52,
                    "`useTitle` calls no hooks, so the React Compiler skips it and whatever it returns is rebuilt on every render. Rename it to a plain function, or call the hook it depends on inside it.",
                )],
            },
            Case {
                name: "composable outside React",
                path: "case.ts",
                code: "export function useTitle(name: string): string {\n\treturn name;\n}\n",
                expected: &[],
            },
            Case { name: "not a hook name", path: "case.tsx", code: "export function user(): string {\n\treturn \"\";\n}\n", expected: &[] },
            Case { name: "bare use is not a hook name", path: "case.tsx", code: "export function use(): string {\n\treturn \"\";\n}\n", expected: &[] },
        ],
    );
}
