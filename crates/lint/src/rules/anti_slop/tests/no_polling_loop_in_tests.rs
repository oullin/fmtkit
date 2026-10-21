//! `anti-slop/no-polling-loop-in-tests`: adapted from t3code's `t3code/no-polling-loop-in-tests`.

use super::{Case, check};

/// The rule's behaviour, written for fmtkit rather than recorded from t3code's plugin.
#[test]
fn adapted() {
    check(
        "anti-slop/no-polling-loop-in-tests",
        "[]",
        &[
            Case {
                name: "sleep until a predicate",
                path: "case.test.ts",
                code: "it(\"x\", async () => {\n\twhile (!done()) {\n\t\tawait new Promise((resolve) => setTimeout(resolve, 10));\n\t}\n});\n",
                expected: &[(
                    23,
                    102,
                    "Polling loop in a test: spinning on a tick or a sleep depends on the scheduler and runner load. Await the promise, event, or callback that marks the milestone, or use `expect.poll`.",
                )],
            },
            Case {
                name: "tick until a break",
                path: "case.test.ts",
                code: "for (;;) {\n\tif (ready()) {\n\t\tbreak;\n\t}\n\n\tawait Promise.resolve();\n}\n",
                expected: &[(
                    0,
                    67,
                    "Polling loop in a test: spinning on a tick or a sleep depends on the scheduler and runner load. Await the promise, event, or callback that marks the milestone, or use `expect.poll`.",
                )],
            },
            Case {
                name: "counter nothing advances",
                path: "case.test.ts",
                code: "let tries = 0;\n\nwhile (tries < 5) {\n\tawait Promise.resolve();\n}\n",
                expected: &[(
                    16,
                    63,
                    "Polling loop in a test: spinning on a tick or a sleep depends on the scheduler and runner load. Await the promise, event, or callback that marks the milestone, or use `expect.poll`.",
                )],
            },
            Case {
                name: "promise timer",
                path: "case.test.ts",
                code: "import { setTimeout as sleep } from \"node:timers/promises\";\n\nwhile (!done()) {\n\tawait sleep(5);\n}\n",
                expected: &[(
                    61,
                    97,
                    "Polling loop in a test: spinning on a tick or a sleep depends on the scheduler and runner load. Await the promise, event, or callback that marks the milestone, or use `expect.poll`.",
                )],
            },
            Case {
                name: "setImmediate on a predicate",
                path: "case.test.ts",
                code: "do {\n\tsetImmediate(tick);\n} while (pending.length > 0);\n",
                expected: &[(
                    0,
                    55,
                    "Polling loop in a test: spinning on a tick or a sleep depends on the scheduler and runner load. Await the promise, event, or callback that marks the milestone, or use `expect.poll`.",
                )],
            },
            Case { name: "fixed flushes", path: "case.test.ts", code: "for (let i = 0; i < 3; i++) {\n\tawait Promise.resolve();\n}\n", expected: &[] },
            Case {
                name: "advanced counter",
                path: "case.test.ts",
                code: "let attempt = 0;\n\nwhile (attempt < max) {\n\tawait Promise.resolve();\n\tattempt += 1;\n}\n",
                expected: &[],
            },
            Case { name: "no wait", path: "case.test.ts", code: "while (!done()) {\n\tstep();\n}\n", expected: &[] },
            Case {
                name: "wait in a callback",
                path: "case.test.ts",
                code: "while (!done()) {\n\tqueue.push(async () => {\n\t\tawait Promise.resolve();\n\t});\n}\n",
                expected: &[],
            },
            Case { name: "loop over items", path: "case.test.ts", code: "for (const item of items) {\n\tawait Promise.resolve(item);\n}\n", expected: &[] },
            Case { name: "not a test file", path: "case.ts", code: "while (!done()) {\n\tawait Promise.resolve();\n}\n", expected: &[] },
        ],
    );
}
