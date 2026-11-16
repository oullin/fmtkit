//! `anti-slop-martinfowler/mysterious-name`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/martinfowler/rules/mysterious-name.test.ts`.

use super::super::super::anti_slop::tests::{Case, check};

/// src/martinfowler/rules/mysterious-name.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop-martinfowler/mysterious-name",
        "[]",
        &[
            Case {
                name: "valid 0: export function loadAndReconcileConfigurationSources() {}",
                path: "file.ts",
                code: "export function loadAndReconcileConfigurationSources() {}",
                expected: &[],
            },
            Case {
                name: "valid 1: export const getCachedOrAcquireCredential = () => cache.get()",
                path: "file.ts",
                code: "export const getCachedOrAcquireCredential = () => cache.get();",
                expected: &[],
            },
            Case { name: "valid 2: export const handle = openFile(path);", path: "file.ts", code: "export const handle = openFile(path);", expected: &[] },
            Case { name: "valid 3: const handle = { close() {} };", path: "file.ts", code: "const handle = { close() {} };", expected: &[] },
            Case {
                name: "valid 4: export const make = Effect.gen(function* () { return {} });",
                path: "file.ts",
                code: "export const make = Effect.gen(function* () { return {} });",
                expected: &[],
            },
            Case {
                name: "valid 5: export function fetch(request: Request) { return respond(requ",
                path: "file.ts",
                code: "export function fetch(request: Request) { return respond(request); }",
                expected: &[],
            },
            Case {
                name: "valid 6: export const layer = Layer.effect(Service, make);",
                path: "file.ts",
                code: "export const layer = Layer.effect(Service, make);",
                expected: &[],
            },
            Case {
                name: "valid 7: export function syncMailbox() { const run = () => step(); run",
                path: "file.ts",
                code: "export function syncMailbox() { const run = () => step(); run(); }",
                expected: &[],
            },
            Case { name: "valid 8: export function processRefund() {}", path: "file.ts", code: "export function processRefund() {}", expected: &[] },
            Case {
                name: "valid 9: export const runMigrations = Effect.fn('db.runMigrations')(fu",
                path: "file.ts",
                code: "export const runMigrations = Effect.fn('db.runMigrations')(function* () {});",
                expected: &[],
            },
            Case {
                name: "valid 10: import { load } from './load.ts'; export const loadUser = lo",
                path: "file.ts",
                code: "import { load } from './load.ts'; export const loadUser = load;",
                expected: &[],
            },
            Case {
                name: "valid 11: function a() {} function f() { let current = a; current = b;",
                path: "file.ts",
                code: "function a() {} function f() { let current = a; current = b; return current; }",
                expected: &[],
            },
            Case {
                name: "valid 12: function parseItems(payload) { const raw = payload; return r",
                path: "file.ts",
                code: "function parseItems(payload) { const raw = payload; return raw; }",
                expected: &[],
            },
            Case {
                name: "valid 13: const config = readConfig(); export const settings = config;",
                path: "file.ts",
                code: "const config = readConfig(); export const settings = config;",
                expected: &[],
            },
            Case { name: "valid 14: export function handle() {}", path: "src/handler.test.ts", code: "export function handle() {}", expected: &[] },
            Case { name: "valid 15: export function handle() {}", path: "src/__tests__/handler.ts", code: "export function handle() {}", expected: &[] },
            Case {
                name: "invalid 18: export function resolve() {}",
                path: "file.ts",
                code: "export function resolve() {}",
                expected: &[(
                    16,
                    23,
                    "Possibly Mysterious Name: `resolve` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case {
                name: "invalid 19: export async function process(input: Input) {}",
                path: "file.ts",
                code: "export async function process(input: Input) {}",
                expected: &[(
                    22,
                    29,
                    "Possibly Mysterious Name: `process` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case {
                name: "invalid 20: function handle() {}",
                path: "file.ts",
                code: "function handle() {}",
                expected: &[(
                    9,
                    15,
                    "Possibly Mysterious Name: `handle` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case {
                name: "invalid 21: export default function execute() {}",
                path: "file.ts",
                code: "export default function execute() {}",
                expected: &[(
                    24,
                    31,
                    "Possibly Mysterious Name: `execute` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case {
                name: "invalid 22: export const run = () => step();",
                path: "file.ts",
                code: "export const run = () => step();",
                expected: &[(
                    13,
                    16,
                    "Possibly Mysterious Name: `run` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case {
                name: "invalid 23: export const doWork = async () => {};",
                path: "file.ts",
                code: "export const doWork = async () => {};",
                expected: &[(
                    13,
                    19,
                    "Possibly Mysterious Name: `doWork` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case {
                name: "invalid 24: const processData = function (rows) {};",
                path: "file.ts",
                code: "const processData = function (rows) {};",
                expected: &[(
                    6,
                    17,
                    "Possibly Mysterious Name: `processData` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case {
                name: "invalid 25: export const update = Effect.fn('slots.update')(function* ",
                path: "file.ts",
                code: "export const update = Effect.fn('slots.update')(function* () {});",
                expected: &[(
                    13,
                    19,
                    "Possibly Mysterious Name: `update` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case {
                name: "invalid 26: export const sync = Effect.fn(function* () {});",
                path: "file.ts",
                code: "export const sync = Effect.fn(function* () {});",
                expected: &[(
                    13,
                    17,
                    "Possibly Mysterious Name: `sync` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case {
                name: "invalid 28: \t\t\t\tfunction refreshAuthentication() {}",
                path: "file.ts",
                code: "\n\t\t\t\tfunction refreshAuthentication() {}\n\t\t\t\texport const executeMutation = refreshAuthentication;\n\t\t\t",
                expected: &[(
                    58,
                    97,
                    "Possibly Mysterious Name: `executeMutation` only aliases `refreshAuthentication`, so one behavior has two names and readers must check whether they differ. Consider Rename Function: delete the alias and call `refreshAuthentication` directly, or rename the one function so its name states what it does.",
                )],
            },
            Case {
                name: "invalid 29: \t\t\t\tconst make = Effect.gen(function* () {",
                path: "file.ts",
                code: "\n\t\t\t\tconst make = Effect.gen(function* () {\n\t\t\t\t\tconst refreshAuthentication = (id, procedure) => run(id, procedure);\n\t\t\t\t\tconst executeMutation = refreshAuthentication;\n\t\t\t\t\treturn { executeMutation };\n\t\t\t\t});\n\t\t\t",
                expected: &[(
                    129,
                    168,
                    "Possibly Mysterious Name: `executeMutation` only aliases `refreshAuthentication`, so one behavior has two names and readers must check whether they differ. Consider Rename Function: delete the alias and call `refreshAuthentication` directly, or rename the one function so its name states what it does.",
                )],
            },
            Case {
                name: "invalid 30: \t\t\t\tconst loadSlot = Effect.fn(\"slot.load\")(function* () {",
                path: "file.ts",
                code: "\n\t\t\t\tconst loadSlot = Effect.fn(\"slot.load\")(function* () {});\n\t\t\t\tconst fetchSlot = loadSlot;\n\t\t\t",
                expected: &[(
                    73,
                    93,
                    "Possibly Mysterious Name: `fetchSlot` only aliases `loadSlot`, so one behavior has two names and readers must check whether they differ. Consider Rename Function: delete the alias and call `loadSlot` directly, or rename the one function so its name states what it does.",
                )],
            },
        ],
    );
}

/// src/martinfowler/rules/mysterious-name.test.ts, options `[{"allowNames":["handle"]}]`
#[test]
fn upstream_options_1() {
    check(
        "anti-slop-martinfowler/mysterious-name",
        "[{\"allowNames\":[\"handle\"]}]",
        &[Case { name: "valid 16: export function handle() {}", path: "file.ts", code: "export function handle() {}", expected: &[] }],
    );
}

/// src/martinfowler/rules/mysterious-name.test.ts, options `[{"verbs":[],"checkAliases":false}]`
#[test]
fn upstream_options_2() {
    check(
        "anti-slop-martinfowler/mysterious-name",
        "[{\"verbs\":[],\"checkAliases\":false}]",
        &[Case {
            name: "valid 17: const refresh = () => 1; export const run = refresh;",
            path: "file.ts",
            code: "const refresh = () => 1; export const run = refresh;",
            expected: &[],
        }],
    );
}

/// src/martinfowler/rules/mysterious-name.test.ts, options `[{"verbs":["load"]}]`
#[test]
fn upstream_options_3() {
    check(
        "anti-slop-martinfowler/mysterious-name",
        "[{\"verbs\":[\"load\"]}]",
        &[Case {
            name: "invalid 27: export function load() {}",
            path: "file.ts",
            code: "export function load() {}",
            expected: &[(
                16,
                20,
                "Possibly Mysterious Name: `load` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
            )],
        }],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/mysterious-name.mfedges.test.ts
#[test]
fn edges() {
    check(
        "anti-slop-martinfowler/mysterious-name",
        "[]",
        &[
            Case {
                name: "invalid 0: export const run: Fn = () => step();",
                path: "file.ts",
                code: "export const run: Fn = () => step();",
                expected: &[(
                    13,
                    20,
                    "Possibly Mysterious Name: `run` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case {
                name: "invalid 1: function processItems() {}",
                path: "file.ts",
                code: "function processItems() {}",
                expected: &[(
                    9,
                    21,
                    "Possibly Mysterious Name: `processItems` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case {
                name: "invalid 2: function updateValue() {}",
                path: "file.ts",
                code: "function updateValue() {}",
                expected: &[(
                    9,
                    20,
                    "Possibly Mysterious Name: `updateValue` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case {
                name: "invalid 3: function initialize() {}",
                path: "file.ts",
                code: "function initialize() {}",
                expected: &[(
                    9,
                    19,
                    "Possibly Mysterious Name: `initialize` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case {
                name: "invalid 4: function doAll() {}",
                path: "file.ts",
                code: "function doAll() {}",
                expected: &[(
                    9,
                    14,
                    "Possibly Mysterious Name: `doAll` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case { name: "invalid 5: function processdata() {}", path: "file.ts", code: "function processdata() {}", expected: &[] },
            Case {
                name: "invalid 6: function handleIt() {}",
                path: "file.ts",
                code: "function handleIt() {}",
                expected: &[(
                    9,
                    17,
                    "Possibly Mysterious Name: `handleIt` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case { name: "invalid 7: export default function () {}", path: "file.ts", code: "export default function () {}", expected: &[] },
            Case {
                name: "invalid 8: function handle(): void;",
                path: "file.ts",
                code: "function handle(): void;\nfunction handle() {}",
                expected: &[(
                    34,
                    40,
                    "Possibly Mysterious Name: `handle` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case { name: "invalid 9: declare function run(): void;", path: "file.ts", code: "declare function run(): void;", expected: &[] },
            Case { name: "invalid 10: export declare function run(): void;", path: "file.ts", code: "export declare function run(): void;", expected: &[] },
            Case {
                name: "invalid 11: let run = function () {};",
                path: "file.ts",
                code: "let run = function () {};",
                expected: &[(
                    4,
                    7,
                    "Possibly Mysterious Name: `run` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case {
                name: "invalid 12: var run = () => 1, sync = 2;",
                path: "file.ts",
                code: "var run = () => 1, sync = 2;",
                expected: &[(
                    4,
                    7,
                    "Possibly Mysterious Name: `run` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case {
                name: "invalid 13: const run = (() => 1) as Fn;",
                path: "file.ts",
                code: "const run = (() => 1) as Fn;",
                expected: &[(
                    6,
                    9,
                    "Possibly Mysterious Name: `run` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case {
                name: "invalid 14: const run = (function () {});",
                path: "file.ts",
                code: "const run = (function () {});",
                expected: &[(
                    6,
                    9,
                    "Possibly Mysterious Name: `run` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case { name: "invalid 15: class run {}", path: "file.ts", code: "class run {}", expected: &[] },
            Case {
                name: "invalid 16: const run = Effect.fnUntraced(function* () {});",
                path: "file.ts",
                code: "const run = Effect.fnUntraced(function* () {});",
                expected: &[(
                    6,
                    9,
                    "Possibly Mysterious Name: `run` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case {
                name: "invalid 17: const run = Effect.fn('x', { a: 1 })(function* () {});",
                path: "file.ts",
                code: "const run = Effect.fn('x', { a: 1 })(function* () {});",
                expected: &[(
                    6,
                    9,
                    "Possibly Mysterious Name: `run` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case {
                name: "invalid 18: const run = Effect.gen(function* () {});",
                path: "file.ts",
                code: "const run = Effect.gen(function* () {});",
                expected: &[],
            },
            Case {
                name: "invalid 19: const run = other.fn(() => 1);",
                path: "file.ts",
                code: "const run = other.fn(() => 1);",
                expected: &[(
                    6,
                    9,
                    "Possibly Mysterious Name: `run` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case { name: "invalid 20: const run = other.fn(...fns);", path: "file.ts", code: "const run = other.fn(...fns);", expected: &[] },
            Case { name: "invalid 21: const run = other['fn'](() => 1);", path: "file.ts", code: "const run = other['fn'](() => 1);", expected: &[] },
            Case { name: "invalid 22: const { run } = { run: () => 1 };", path: "file.ts", code: "const { run } = { run: () => 1 };", expected: &[] },
            Case {
                name: "invalid 23: function outer() { function run() {} const sync = () => 1;",
                path: "file.ts",
                code: "function outer() { function run() {} const sync = () => 1; }",
                expected: &[],
            },
            Case {
                name: "invalid 24: export default run; function run() {}",
                path: "file.ts",
                code: "export default run; function run() {}",
                expected: &[(
                    29,
                    32,
                    "Possibly Mysterious Name: `run` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                )],
            },
            Case {
                name: "invalid 25: const a = Effect.fnUntraced(function* () {}); export const",
                path: "file.ts",
                code: "const a = Effect.fnUntraced(function* () {}); export const go = a;",
                expected: &[(
                    59,
                    65,
                    "Possibly Mysterious Name: `go` only aliases `a`, so one behavior has two names and readers must check whether they differ. Consider Rename Function: delete the alias and call `a` directly, or rename the one function so its name states what it does.",
                )],
            },
            Case { name: "invalid 26: import { f } from 'f'; const g = f;", path: "file.ts", code: "import { f } from 'f'; const g = f;", expected: &[] },
            Case { name: "invalid 27: class C {} const D = C;", path: "file.ts", code: "class C {} const D = C;", expected: &[] },
            Case { name: "invalid 28: const x = 1; const y = x;", path: "file.ts", code: "const x = 1; const y = x;", expected: &[] },
            Case {
                name: "invalid 29: function f() {} const x = (f as any);",
                path: "file.ts",
                code: "function f() {} const x = (f as any);",
                expected: &[(
                    22,
                    36,
                    "Possibly Mysterious Name: `x` only aliases `f`, so one behavior has two names and readers must check whether they differ. Consider Rename Function: delete the alias and call `f` directly, or rename the one function so its name states what it does.",
                )],
            },
            Case {
                name: "invalid 30: function f() {} const x = f!;",
                path: "file.ts",
                code: "function f() {} const x = f!;",
                expected: &[(
                    22,
                    28,
                    "Possibly Mysterious Name: `x` only aliases `f`, so one behavior has two names and readers must check whether they differ. Consider Rename Function: delete the alias and call `f` directly, or rename the one function so its name states what it does.",
                )],
            },
            Case { name: "invalid 31: function f() {} let x = f;", path: "file.ts", code: "function f() {} let x = f;", expected: &[] },
            Case {
                name: "invalid 32: var f = function () {}; const x = f;",
                path: "file.ts",
                code: "var f = function () {}; const x = f;",
                expected: &[(
                    30,
                    35,
                    "Possibly Mysterious Name: `x` only aliases `f`, so one behavior has two names and readers must check whether they differ. Consider Rename Function: delete the alias and call `f` directly, or rename the one function so its name states what it does.",
                )],
            },
            Case {
                name: "invalid 33: function f() {} { const g = f; }",
                path: "file.ts",
                code: "function f() {} { const g = f; }",
                expected: &[(
                    24,
                    29,
                    "Possibly Mysterious Name: `g` only aliases `f`, so one behavior has two names and readers must check whether they differ. Consider Rename Function: delete the alias and call `f` directly, or rename the one function so its name states what it does.",
                )],
            },
            Case { name: "invalid 34: function f(cb) { const g = cb; }", path: "file.ts", code: "function f(cb) { const g = cb; }", expected: &[] },
            Case {
                name: "invalid 35: function f() {} const f2 = f, f3 = f;",
                path: "file.ts",
                code: "function f() {} const f2 = f, f3 = f;",
                expected: &[
                    (
                        22,
                        28,
                        "Possibly Mysterious Name: `f2` only aliases `f`, so one behavior has two names and readers must check whether they differ. Consider Rename Function: delete the alias and call `f` directly, or rename the one function so its name states what it does.",
                    ),
                    (
                        30,
                        36,
                        "Possibly Mysterious Name: `f3` only aliases `f`, so one behavior has two names and readers must check whether they differ. Consider Rename Function: delete the alias and call `f` directly, or rename the one function so its name states what it does.",
                    ),
                ],
            },
            Case {
                name: "invalid 36: function f() {} function g() { const f = 1; const h = f; }",
                path: "file.ts",
                code: "function f() {} function g() { const f = 1; const h = f; }",
                expected: &[],
            },
            Case { name: "invalid 37: function f(): void;", path: "file.ts", code: "function f(): void;\nfunction f() {}\nconst g = f;", expected: &[] },
            Case {
                name: "invalid 38: declare function f(): void; const g = f;",
                path: "file.ts",
                code: "declare function f(): void; const g = f;",
                expected: &[],
            },
            Case {
                name: "invalid 39: const f = () => 1; const f2 = f; const f3 = f2;",
                path: "file.ts",
                code: "const f = () => 1; const f2 = f; const f3 = f2;",
                expected: &[(
                    25,
                    31,
                    "Possibly Mysterious Name: `f2` only aliases `f`, so one behavior has two names and readers must check whether they differ. Consider Rename Function: delete the alias and call `f` directly, or rename the one function so its name states what it does.",
                )],
            },
            Case {
                name: "invalid 40: function f() {} const g: typeof f = f;",
                path: "file.ts",
                code: "function f() {} const g: typeof f = f;",
                expected: &[(
                    22,
                    37,
                    "Possibly Mysterious Name: `g` only aliases `f`, so one behavior has two names and readers must check whether they differ. Consider Rename Function: delete the alias and call `f` directly, or rename the one function so its name states what it does.",
                )],
            },
            Case { name: "invalid 44: export function run() {}", path: "src/__mocks__/a.ts", code: "export function run() {}", expected: &[] },
        ],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/mysterious-name.mfedges.test.ts, options `[{"vagueNouns":["Stuff"]}]`
#[test]
fn edges_options_1() {
    check(
        "anti-slop-martinfowler/mysterious-name",
        "[{\"vagueNouns\":[\"Stuff\"]}]",
        &[Case {
            name: "invalid 41: function processData() {} function processStuff() {}",
            path: "file.ts",
            code: "function processData() {} function processStuff() {}",
            expected: &[(
                35,
                47,
                "Possibly Mysterious Name: `processStuff` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
            )],
        }],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/mysterious-name.mfedges.test.ts, options `[{"verbs":["fetch","make"],"allowNames":[]}]`
#[test]
fn edges_options_2() {
    check(
        "anti-slop-martinfowler/mysterious-name",
        "[{\"verbs\":[\"fetch\",\"make\"],\"allowNames\":[]}]",
        &[Case {
            name: "invalid 42: function fetch() {} function make() {}",
            path: "file.ts",
            code: "function fetch() {} function make() {}",
            expected: &[
                (
                    9,
                    14,
                    "Possibly Mysterious Name: `fetch` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                ),
                (
                    29,
                    33,
                    "Possibly Mysterious Name: `make` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
                ),
            ],
        }],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/mysterious-name.mfedges.test.ts, options `[{"verbs":["go"],"vagueNouns":[""]}]`
#[test]
fn edges_options_3() {
    check(
        "anti-slop-martinfowler/mysterious-name",
        "[{\"verbs\":[\"go\"],\"vagueNouns\":[\"\"]}]",
        &[Case {
            name: "invalid 43: function go() {} function goAll() {}",
            path: "file.ts",
            code: "function go() {} function goAll() {}",
            expected: &[(
                9,
                11,
                "Possibly Mysterious Name: `go` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
            )],
        }],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/mysterious-name.mfedges.test.ts, options `[{"ignoreTests":false}]`
#[test]
fn edges_options_4() {
    check(
        "anti-slop-martinfowler/mysterious-name",
        "[{\"ignoreTests\":false}]",
        &[Case {
            name: "invalid 45: export function run() {}",
            path: "src/__mocks__/a.ts",
            code: "export function run() {}",
            expected: &[(
                16,
                19,
                "Possibly Mysterious Name: `run` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`).",
            )],
        }],
    );
}
