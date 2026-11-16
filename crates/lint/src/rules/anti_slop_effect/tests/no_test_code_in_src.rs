//! `anti-slop-effect/no-test-code-in-src`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/effect/rules/no-test-code-in-src.test.ts`.

use super::super::super::anti_slop::tests::{Case, check};

/// src/effect/rules/no-test-code-in-src.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop-effect/no-test-code-in-src",
        "[]",
        &[
            Case {
                name: "valid 0: export const layer = Widgets.layer;",
                path: "repo/packages/widgets/src/widgets.ts",
                code: "export const layer = Widgets.layer;\nexport const hitTest = (x: number) => x > 0;",
                expected: &[],
            },
            Case {
                name: "valid 1: /** Stands in for the billing service until it ships. */",
                path: "repo/packages/widgets/src/widgets.ts",
                code: "/** Stands in for the billing service until it ships. */\nexport const layerFixtures = Layer.succeed(Billing, fixtures);",
                expected: &[],
            },
            Case {
                name: "valid 2: export const TestConnection = () => null;",
                path: "repo/packages/widgets/src/widgets.ts",
                code: "export const TestConnection = () => null;\nexport class Widgets { static readonly layer = live; }",
                expected: &[],
            },
            Case {
                name: "valid 3: import { widget } from \"./latest/widget.ts\";",
                path: "repo/packages/widgets/src/widgets.ts",
                code: "import { widget } from \"./latest/widget.ts\";\nimport { helper } from \"test-utils\";",
                expected: &[],
            },
            Case {
                name: "valid 4: export const layerTest = Widgets.layer;",
                path: "repo/packages/widgets/test/support/layers.ts",
                code: "export const layerTest = Widgets.layer;\nimport { x } from \"../../test/x.ts\";",
                expected: &[],
            },
            Case {
                name: "valid 5: export const FakeClock = {};",
                path: "repo/packages/widgets/test/widgets.test.ts",
                code: "export const FakeClock = {};",
                expected: &[],
            },
            Case {
                name: "invalid 8: it(\"works\", () => {});",
                path: "repo/packages/widgets/src/widgets.test.ts",
                code: "it(\"works\", () => {});",
                expected: &[(
                    0,
                    22,
                    "This test file lives in production source (`src/`). Move it to the package's test/ directory, with shared helpers in test/support/.",
                )],
            },
            Case {
                name: "invalid 9: export const value = 1;",
                path: "repo/packages/widgets/src/__tests__/helpers.ts",
                code: "export const value = 1;",
                expected: &[(
                    0,
                    23,
                    "This test file lives in production source (`src/`). Move it to the package's test/ directory, with shared helpers in test/support/.",
                )],
            },
            Case {
                name: "invalid 10: import { fake } from \"../test/support/fakes.ts\";",
                path: "repo/packages/widgets/src/widgets.ts",
                code: "import { fake } from \"../test/support/fakes.ts\";\nexport * from \"./__tests__/helpers.ts\";\nconst lazy = import(\"../tests/lazy.ts\");",
                expected: &[
                    (
                        21,
                        47,
                        "Production source imports `../test/support/fakes.ts` from a test directory. Production modules never depend on test code: move what it needs into source, or move this module to test/.",
                    ),
                    (
                        63,
                        87,
                        "Production source imports `./__tests__/helpers.ts` from a test directory. Production modules never depend on test code: move what it needs into source, or move this module to test/.",
                    ),
                    (
                        109,
                        127,
                        "Production source imports `../tests/lazy.ts` from a test directory. Production modules never depend on test code: move what it needs into source, or move this module to test/.",
                    ),
                ],
            },
            Case {
                name: "invalid 11: \t\t\t\texport const layerTest = Widgets.layer;",
                path: "repo/packages/widgets/src/widgets.ts",
                code: "\n\t\t\t\texport const layerTest = Widgets.layer;\n\t\t\t\texport function makeFakeWidgets() {}\n\t\t\t\texport class FakeClock {}\n\t\t\t\texport const WidgetsTestLayer = live;\n\t\t\t\tconst recorder = {};\n\t\t\t\texport { recorder as WidgetsStub };\n\t\t\t",
                expected: &[
                    (
                        18,
                        27,
                        "`layerTest` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
                    ),
                    (
                        65,
                        80,
                        "`makeFakeWidgets` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
                    ),
                    (
                        103,
                        112,
                        "`FakeClock` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
                    ),
                    (
                        133,
                        149,
                        "`WidgetsTestLayer` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
                    ),
                    (
                        196,
                        219,
                        "`WidgetsStub` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
                    ),
                ],
            },
            Case {
                name: "invalid 12: export class Widgets { static readonly layerMock = mock; s",
                path: "repo/packages/widgets/src/widgets.ts",
                code: "export class Widgets { static readonly layerMock = mock; static layerStub() {} }",
                expected: &[
                    (
                        39,
                        48,
                        "`layerMock` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
                    ),
                    (
                        64,
                        73,
                        "`layerStub` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
                    ),
                ],
            },
        ],
    );
}

/// src/effect/rules/no-test-code-in-src.test.ts, options `[{"sourceDirectories":["lib"]}]`
#[test]
fn upstream_options_1() {
    check(
        "anti-slop-effect/no-test-code-in-src",
        "[{\"sourceDirectories\":[\"lib\"]}]",
        &[
            Case {
                name: "valid 6: export const layerTest = Widgets.layer;",
                path: "repo/packages/widgets/src/widgets.ts",
                code: "export const layerTest = Widgets.layer;",
                expected: &[],
            },
            Case {
                name: "invalid 13: export const layerTest = Widgets.layer;",
                path: "repo/lib/widgets.ts",
                code: "export const layerTest = Widgets.layer;",
                expected: &[(
                    13,
                    22,
                    "`layerTest` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
                )],
            },
        ],
    );
}

/// src/effect/rules/no-test-code-in-src.test.ts, options `[{"testNamePatterns":["^layerTest$"]}]`
#[test]
fn upstream_options_2() {
    check(
        "anti-slop-effect/no-test-code-in-src",
        "[{\"testNamePatterns\":[\"^layerTest$\"]}]",
        &[Case {
            name: "valid 7: export const WidgetsMock = {};",
            path: "repo/packages/widgets/src/widgets.ts",
            code: "export const WidgetsMock = {};",
            expected: &[],
        }],
    );
}

/// Edge cases the upstream tests miss, recorded from the upstream plugin.
#[test]
fn edges() {
    check(
        "anti-slop-effect/no-test-code-in-src",
        "[]",
        &[
            Case { name: "valid 0: export const layerTest = 1;", path: "lib/store.test.ts", code: "export const layerTest = 1;", expected: &[] },
            Case {
                name: "valid 1: import { a } from \"test/helpers.ts\"; import { b } from \"./tes",
                path: "src/store.ts",
                code: "import { a } from \"test/helpers.ts\"; import { b } from \"./testing/b.ts\";",
                expected: &[],
            },
            Case {
                name: "valid 2: export const layer = 1; class A { static FakeClock = 1; [`Fak",
                path: "src/store.ts",
                code: "export const layer = 1; class A { static FakeClock = 1; [`FakeClock`] = 2; static ['MockStore'] = 3; }",
                expected: &[(
                    41,
                    50,
                    "`FakeClock` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
                )],
            },
            Case {
                name: "valid 3: export declare function makeTestStore(): void;",
                path: "src/store.ts",
                code: "export declare function makeTestStore(): void;",
                expected: &[],
            },
            Case { name: "valid 4: export const value = 1;", path: "src/test.ts", code: "export const value = 1;", expected: &[] },
            Case {
                name: "invalid 5: // leading comment",
                path: "src/store.test.ts",
                code: "// leading comment\nexport const value = 1;\n",
                expected: &[(
                    19,
                    43,
                    "This test file lives in production source (`src/`). Move it to the package's test/ directory, with shared helpers in test/support/.",
                )],
            },
            Case {
                name: "invalid 6: const value = 1; // trailing",
                path: "packages/app/src/feature/__tests__/store.ts",
                code: "\n\nconst value = 1; // trailing\n",
                expected: &[(
                    2,
                    31,
                    "This test file lives in production source (`src/`). Move it to the package's test/ directory, with shared helpers in test/support/.",
                )],
            },
            Case {
                name: "invalid 7: export const layerTest: Layer.Layer<Store> = make, other = ",
                path: "src/store.ts",
                code: "export const layerTest: Layer.Layer<Store> = make, other = 1, StoreMock = 2;",
                expected: &[
                    (
                        13,
                        42,
                        "`layerTest` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
                    ),
                    (
                        62,
                        71,
                        "`StoreMock` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
                    ),
                ],
            },
            Case {
                name: "invalid 8: export { layer as layerFake, FakeClock } from './clock.ts';",
                path: "src/store.ts",
                code: "export { layer as layerFake, FakeClock } from './clock.ts'; const x = 1; export { x as MockX };",
                expected: &[
                    (
                        9,
                        27,
                        "`layerFake` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
                    ),
                    (
                        29,
                        38,
                        "`FakeClock` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
                    ),
                    (
                        82,
                        92,
                        "`MockX` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
                    ),
                ],
            },
            Case {
                name: "invalid 9: const a = import((\"../tests/a.ts\")); export * from \"./__tes",
                path: "src/store.ts",
                code: "const a = import((\"../tests/a.ts\")); export * from \"./__tests__/b.ts\"; import \"../test/c.ts\";",
                expected: &[
                    (
                        18,
                        33,
                        "Production source imports `../tests/a.ts` from a test directory. Production modules never depend on test code: move what it needs into source, or move this module to test/.",
                    ),
                    (
                        51,
                        69,
                        "Production source imports `./__tests__/b.ts` from a test directory. Production modules never depend on test code: move what it needs into source, or move this module to test/.",
                    ),
                    (
                        78,
                        92,
                        "Production source imports `../test/c.ts` from a test directory. Production modules never depend on test code: move what it needs into source, or move this module to test/.",
                    ),
                ],
            },
            Case {
                name: "invalid 10: export class FakeStore { static readonly layerMock = 1; st",
                path: "src/store.ts",
                code: "export class FakeStore { static readonly layerMock = 1; static #StubThing = 2; static makeFakeStore() {} }",
                expected: &[
                    (
                        13,
                        22,
                        "`FakeStore` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
                    ),
                    (
                        41,
                        50,
                        "`layerMock` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
                    ),
                    (
                        63,
                        73,
                        "`StubThing` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
                    ),
                    (
                        86,
                        99,
                        "`makeFakeStore` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
                    ),
                ],
            },
            Case {
                name: "invalid 11: export type FakeShape = {}; export interface MockShape {} ",
                path: "src/store.ts",
                code: "export type FakeShape = {}; export interface MockShape {} export enum StubKind {} export function createMockStore() {}",
                expected: &[
                    (
                        12,
                        21,
                        "`FakeShape` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
                    ),
                    (
                        45,
                        54,
                        "`MockShape` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
                    ),
                    (
                        70,
                        78,
                        "`StubKind` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
                    ),
                    (
                        98,
                        113,
                        "`createMockStore` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
                    ),
                ],
            },
            Case {
                name: "program-span invalid 0: /* c */ @dec",
                path: "src/a.test.ts",
                code: "/* c */ @dec\nexport class A {}\n",
                expected: &[(
                    8,
                    31,
                    "This test file lives in production source (`src/`). Move it to the package's test/ directory, with shared helpers in test/support/.",
                )],
            },
            Case {
                name: "program-span invalid 1: // only a comment",
                path: "src/b.test.ts",
                code: "// only a comment\n",
                expected: &[(
                    18,
                    18,
                    "This test file lives in production source (`src/`). Move it to the package's test/ directory, with shared helpers in test/support/.",
                )],
            },
            Case {
                name: "program-span invalid 2: // leading",
                path: "src/c.test.js",
                code: "// leading\nconst value = 1;\n",
                expected: &[(
                    11,
                    28,
                    "This test file lives in production source (`src/`). Move it to the package's test/ directory, with shared helpers in test/support/.",
                )],
            },
            Case {
                name: "program-span invalid 3: \"use strict\";",
                path: "src/d.test.ts",
                code: "\n\"use strict\";\nconst value = 1;\n",
                expected: &[(
                    1,
                    32,
                    "This test file lives in production source (`src/`). Move it to the package's test/ directory, with shared helpers in test/support/.",
                )],
            },
        ],
    );
}

/// Edge cases the upstream tests miss, recorded from the upstream plugin, options `[{"sourceDirectories":["lib"]}]`
#[test]
fn edges_options_1() {
    check(
        "anti-slop-effect/no-test-code-in-src",
        "[{\"sourceDirectories\":[\"lib\"]}]",
        &[Case {
            name: "invalid 12: export const FakeClock = 1;",
            path: "app/lib/src/store.ts",
            code: "export const FakeClock = 1;",
            expected: &[(
                13,
                22,
                "`FakeClock` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
            )],
        }],
    );
}

/// Edge cases the upstream tests miss, recorded from the upstream plugin, options `[{"testDirectories":["spec"],"testNamePatterns":["^Dummy"]}]`
#[test]
fn edges_options_2() {
    check(
        "anti-slop-effect/no-test-code-in-src",
        "[{\"testDirectories\":[\"spec\"],\"testNamePatterns\":[\"^Dummy\"]}]",
        &[Case {
            name: "invalid 13: import \"./spec/a.ts\"; export const DummyClock = 1; export ",
            path: "app/src/spec/store.ts",
            code: "import \"./spec/a.ts\"; export const DummyClock = 1; export const FakeClock = 1;",
            expected: &[
                (0, 78, "This test file lives in production source (`src/`). Move it to the package's test/ directory, with shared helpers in test/support/."),
                (
                    7,
                    20,
                    "Production source imports `./spec/a.ts` from a test directory. Production modules never depend on test code: move what it needs into source, or move this module to test/.",
                ),
                (
                    35,
                    45,
                    "`DummyClock` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({ ... }))))`.",
                ),
            ],
        }],
    );
}

/// Hand-written: options that upstream's schema rejects (oxlint refuses the
/// whole config, so the oracle cannot record them) are an `Err` here.
#[test]
fn options_are_validated() {
    let factory =
        super::super::RULES.iter().find(|(name, _)| *name == "anti-slop-effect/no-test-code-in-src").map(|(_, factory)| *factory).expect("registered");
    let build = |options: &str| factory(&serde_json::from_str::<Vec<serde_json::Value>>(options).expect("valid JSON"));

    assert!(build("[]").is_ok());
    assert!(build("[{}]").is_ok());
    assert!(build(r#"[{"sourceDirectories": ["lib"], "testDirectories": ["spec"], "testNamePatterns": ["^Dummy"]}]"#).is_ok());
    assert!(build(r#"[{"sourceDirectory": ["lib"]}]"#).is_err());
    assert!(build(r#"[{"sourceDirectories": "lib"}]"#).is_err());
    assert!(build(r#"[{"sourceDirectories": [""]}]"#).is_err());
    assert!(build(r#"[{"testDirectories": ["spec", "spec"]}]"#).is_err());
    assert!(build(r#"[{"testDirectories": [1]}]"#).is_err());
    assert!(build(r#"[{"testNamePatterns": ["("]}]"#).is_err());
    assert!(build(r"[true]").is_err());
    assert!(build(r"[{}, {}]").is_err());
}
