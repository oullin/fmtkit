//! `anti-slop/no-real-time-wait-in-tests`: adapted from t3code's `t3code/no-real-time-wait-in-tests`.

use super::{Case, check};

/// The rule's behaviour, written for fmtkit rather than recorded from t3code's plugin.
#[test]
fn adapted() {
    check(
        "anti-slop/no-real-time-wait-in-tests",
        "[]",
        &[
            Case {
                name: "sleep through a promise",
                path: "case.test.ts",
                code: "it(\"waits\", async () => {\n\tawait new Promise((resolve) => setTimeout(resolve, 100));\n});\n",
                expected: &[(
                    58,
                    68,
                    "Real-clock wait in a test: it passes or fails with runner load. Await the promise, event, or callback that marks the milestone, or turn on fake timers and advance them.",
                )],
            },
            Case {
                name: "function executor",
                path: "case.test.ts",
                code: "it(\"waits\", async () => {\n\tawait new Promise(function (resolve) {\n\t\tsetTimeout(resolve, 100);\n\t});\n});\n",
                expected: &[(
                    68,
                    78,
                    "Real-clock wait in a test: it passes or fails with runner load. Await the promise, event, or callback that marks the milestone, or turn on fake timers and advance them.",
                )],
            },
            Case {
                name: "promise timer",
                path: "case.test.ts",
                code: "import { setTimeout as sleep } from \"node:timers/promises\";\n\nit(\"waits\", async () => {\n\tawait sleep(100);\n});\n",
                expected: &[(
                    94,
                    99,
                    "Real-clock wait in a test: it passes or fails with runner load. Await the promise, event, or callback that marks the milestone, or turn on fake timers and advance them.",
                )],
            },
            Case {
                name: "promise timer namespace",
                path: "case.test.ts",
                code: "import * as timers from \"timers/promises\";\n\nit(\"waits\", async () => {\n\tawait timers.setTimeout(10);\n});\n",
                expected: &[(
                    77,
                    94,
                    "Real-clock wait in a test: it passes or fails with runner load. Await the promise, event, or callback that marks the milestone, or turn on fake timers and advance them.",
                )],
            },
            Case {
                name: "timer outside a promise",
                path: "case.test.ts",
                code: "it(\"schedules\", () => {\n\tsetTimeout(() => {}, 10);\n});\n",
                expected: &[],
            },
            Case {
                name: "fake timers",
                path: "case.test.ts",
                code: "vi.useFakeTimers();\n\nit(\"waits\", async () => {\n\tconst done = new Promise((resolve) => setTimeout(resolve, 100));\n\n\tvi.advanceTimersByTime(100);\n\tawait done;\n});\n",
                expected: &[],
            },
            Case {
                name: "not a test file",
                path: "case.ts",
                code: "export const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));\n",
                expected: &[],
            },
        ],
    );
}
