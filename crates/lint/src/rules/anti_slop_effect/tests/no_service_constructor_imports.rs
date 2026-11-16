//! `anti-slop-effect/no-service-constructor-imports`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/effect/rules/no-service-constructor-imports.test.ts`.

use super::super::super::anti_slop::tests::{Case, check};

/// src/effect/rules/no-service-constructor-imports.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop-effect/no-service-constructor-imports",
        "[]",
        &[
            Case {
                name: "valid 0: import { makeIssueService } from \"./issue-service.ts\";",
                path: "src/issue-service.test.ts",
                code: "import { makeIssueService } from \"./issue-service.ts\";",
                expected: &[],
            },
            Case {
                name: "valid 1: import { makeIssueService } from \"../issue-service.ts\";",
                path: "src/issue-service.spec.tsx",
                code: "import { makeIssueService } from \"../issue-service.ts\";",
                expected: &[],
            },
            Case {
                name: "valid 2: import { makeExecutionMemo } from \"alchemy/Runtime/ExecutionM",
                path: "src/runtime.ts",
                code: "import { makeExecutionMemo } from \"alchemy/Runtime/ExecutionMemo\";",
                expected: &[],
            },
            Case {
                name: "valid 3: import { issueServiceLayer } from \"./issue-service.ts\";",
                path: "src/runtime.ts",
                code: "import { issueServiceLayer } from \"./issue-service.ts\";\nWorkspaceName.make(\"name\");",
                expected: &[],
            },
            Case {
                name: "valid 4: import { makeissueService } from \"./issue-service.ts\";",
                path: "src/runtime.ts",
                code: "import { makeissueService } from \"./issue-service.ts\";",
                expected: &[],
            },
            Case {
                name: "invalid 5: import { makeIssueService } from \"./issue-service.ts\";",
                path: "src/runtime.ts",
                code: "import { makeIssueService } from \"./issue-service.ts\";",
                expected: &[(
                    9,
                    25,
                    "Do not import Effect service constructor \"makeIssueService\" into runtime code. Import the owning Layer, yield the contextual service, and allow its requirements to propagate to the composition root.",
                )],
            },
            Case {
                name: "invalid 6: import { makeIssueService as createIssueService } from \"../",
                path: "src/runtime.ts",
                code: "import { makeIssueService as createIssueService } from \"../issue-service.ts\";",
                expected: &[(
                    9,
                    47,
                    "Do not import Effect service constructor \"makeIssueService\" into runtime code. Import the owning Layer, yield the contextual service, and allow its requirements to propagate to the composition root.",
                )],
            },
        ],
    );
}

/// Edge cases the upstream tests miss, recorded from the upstream plugin.
#[test]
fn edges() {
    check(
        "anti-slop-effect/no-service-constructor-imports",
        "[]",
        &[
            Case { name: "valid 0: import { makeStore } from \"store\";", path: "src/app.ts", code: "import { makeStore } from \"store\";", expected: &[] },
            Case {
                name: "valid 1: import { make, makestore, make_store } from \"./store.ts\";",
                path: "src/app.ts",
                code: "import { make, makestore, make_store } from \"./store.ts\";",
                expected: &[],
            },
            Case {
                name: "valid 2: import { makeStore } from \"./store.ts\";",
                path: "src/store.spec.tsx",
                code: "import { makeStore } from \"./store.ts\";",
                expected: &[],
            },
            Case {
                name: "valid 3: import makeStore, * as makeAll from \"./store.ts\";",
                path: "test/app.ts",
                code: "import makeStore, * as makeAll from \"./store.ts\";",
                expected: &[],
            },
            Case {
                name: "invalid 4: import { makeStore as build, type makeType } from \"../store",
                path: "src/app.ts",
                code: "import { makeStore as build, type makeType } from \"../store.ts\";",
                expected: &[
                    (
                        9,
                        27,
                        "Do not import Effect service constructor \"makeStore\" into runtime code. Import the owning Layer, yield the contextual service, and allow its requirements to propagate to the composition root.",
                    ),
                    (
                        29,
                        42,
                        "Do not import Effect service constructor \"makeType\" into runtime code. Import the owning Layer, yield the contextual service, and allow its requirements to propagate to the composition root.",
                    ),
                ],
            },
            Case {
                name: "invalid 5: import { \"makeStore\" as build } from \"./store.ts\";",
                path: "src/app.ts",
                code: "import { \"makeStore\" as build } from \"./store.ts\";",
                expected: &[(
                    9,
                    29,
                    "Do not import Effect service constructor \"makeStore\" into runtime code. Import the owning Layer, yield the contextual service, and allow its requirements to propagate to the composition root.",
                )],
            },
            Case {
                name: "invalid 6: import { makeStore } from \"./store.ts\";",
                path: "test/app.ts",
                code: "import { makeStore } from \"./store.ts\";",
                expected: &[(
                    9,
                    18,
                    "Do not import Effect service constructor \"makeStore\" into runtime code. Import the owning Layer, yield the contextual service, and allow its requirements to propagate to the composition root.",
                )],
            },
        ],
    );
}
