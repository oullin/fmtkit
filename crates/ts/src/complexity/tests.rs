//! The v1 complexity tests (`attribution.test.ts`, `shared-constructs.test.ts`,
//! `line-index.test.ts`, the scoring half of `complexity-command.test.ts`),
//! the shared fixture in `fixtures/complexity/`, and the ESTree-vs-oxc shapes
//! whose expected numbers were taken from v1 itself.

use std::cmp::Ordering;
use std::path::Path;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rustc_hash::FxHashMap;

use fmtkit_core::ComplexityScore;

use super::collate::compare;
use super::lines::LineCursor;
use super::score_program;

fn score(rel: &str, source: &str) -> Vec<ComplexityScore> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(rel).expect("a script extension");
    let parsed = Parser::new(&allocator, source, source_type).parse();

    assert!(parsed.diagnostics.is_empty(), "{rel} does not parse: {:?}", parsed.diagnostics);

    score_program(rel, &parsed.program, source)
}

fn scan(source: &str) -> FxHashMap<String, ComplexityScore> {
    score("shapes.ts", source).into_iter().map(|score| (score.name.clone(), score)).collect()
}

fn sorted_names(scored: &FxHashMap<String, ComplexityScore>) -> Vec<&str> {
    let mut names: Vec<&str> = scored.keys().map(String::as_str).collect();

    names.sort_unstable();

    names
}

/// `(name, line, cyclomatic, cognitive)` in report order.
fn table(rel: &str, source: &str) -> Vec<(String, u32, u32, u32)> {
    score(rel, source).into_iter().map(|score| (score.name, score.line, score.cyclomatic, score.cognitive)).collect()
}

fn rows(expected: &[(&str, u32, u32, u32)]) -> Vec<(String, u32, u32, u32)> {
    expected.iter().map(|&(name, line, cyclomatic, cognitive)| (name.to_owned(), line, cyclomatic, cognitive)).collect()
}

// attribution.test.ts

#[test]
fn every_function_like_shape_reports_under_the_name_that_declares_it() {
    let scored = scan(
        "
		function declared(): void {}

		const assigned = function (): void {};

		const arrow = (): void => {};

		class Widget {
			constructor() {}

			method(): void {}

			get size(): number {
				return 0;
			}

			set size(value: number) {}

			field = (): void => {};
		}

		const literal = {
			property(): void {},
		};

		let later: () => void;

		later = function (): void {};
	",
    );

    let mut expected =
        vec!["Widget.constructor", "Widget.field", "Widget.get size", "Widget.method", "Widget.set size", "arrow", "assigned", "declared", "later", "property"];

    expected.sort_unstable();

    assert_eq!(sorted_names(&scored), expected);
}

#[test]
fn an_anonymous_callback_folds_its_cognitive_cost_into_the_named_owner() {
    let scored = scan(
        "
		export function owner(rows: number[][]): number[][] {
			return rows.map((row) => {
				return row.filter((cell) => {
					if (cell > 0) {
						return true;
					}

					return false;
				});
			});
		}
	",
    );

    assert_eq!(sorted_names(&scored), ["owner"]);

    // Two closures deepen the nesting; the if then costs 1 + 2.
    assert_eq!(scored["owner"].cognitive, 3);
}

#[test]
fn a_callback_keeps_its_own_cyclomatic_number_and_the_key_reports_the_worst() {
    let scored = scan(
        "
		export function owner(rows: number[]): number[] {
			return rows.filter((cell) => {
				if (cell > 0 && cell < 10) {
					return true;
				}

				return false;
			});
		}
	",
    );

    // The declaration itself branches nowhere; its callback scores 1 + if + &&.
    assert_eq!(scored["owner"].cyclomatic, 3);
}

#[test]
fn a_nested_named_function_is_reported_on_its_own_as_well_as_folded_in() {
    let scored = scan(
        "
		export function outer(a: number): number {
			function inner(b: number): number {
				if (b > 0) {
					return b;
				}

				return 0;
			}

			return inner(a);
		}
	",
    );

    assert_eq!(scored["inner"].cognitive, 1);
    assert_eq!(scored["outer"].cognitive, 2);
}

#[test]
fn two_declarations_sharing_a_name_are_kept_apart_by_their_line() {
    let scored = scan(
        "
		const first = {
			handler(): void {},
		};

		const second = {
			handler(): void {},
		};
	",
    );

    assert_eq!(sorted_names(&scored), ["handler", "handler:7"]);
}

#[test]
fn an_anonymous_export_default_reports_under_default() {
    let scored = scan(
        "
		export default function (value: number): number {
			return value;
		}
	",
    );

    assert!(scored.contains_key("default"));
}

#[test]
fn a_function_no_declaration_reaches_reports_under_anonymous() {
    let scored = scan(
        "
		[1].map(function (value) {
			return value > 0 ? 1 : 0;
		});
	",
    );

    assert_eq!(scored["<anonymous>"].cyclomatic, 2);
}

// shared-constructs.test.ts

/// The v1 test's template literal, leading newline included, so `ifChain`
/// starts on line 2 as the v1 test asserts.
fn shared_constructs() -> String {
    format!("\n{}", include_str!("../../../../fixtures/complexity/shapes.ts"))
}

#[test]
fn the_shared_shapes_score_the_same_as_their_go_twins() {
    let scored = scan(&shared_constructs());

    let shared = [
        ("ifChain", 4, 3),
        ("elseIfLadder", 4, 4),
        ("switchFour", 5, 1),
        ("nestedClosure", 2, 2),
        ("logicalRun", 4, 1),
        ("mixedLogical", 3, 2),
        ("loopWithIf", 3, 3),
    ];

    for (name, cyclomatic, cognitive) in shared {
        let score = scored.get(name).unwrap_or_else(|| panic!("{name} was not scored"));

        assert_eq!(score.cyclomatic, cyclomatic, "{name} cyclomatic");
        assert_eq!(score.cognitive, cognitive, "{name} cognitive");
    }
}

#[test]
fn a_catch_clause_costs_one_on_both_metrics() {
    let scored = scan(
        "
		export function tryCatch(run: () => void): number {
			try {
				run();

				return 0;
			} catch {
				return 1;
			}
		}
	",
    );

    assert_eq!(scored["tryCatch"].cyclomatic, 2);
    assert_eq!(scored["tryCatch"].cognitive, 1);
}

#[test]
fn a_ternary_counts_like_an_if() {
    let scored = scan(
        "
		export function ternary(a: number): number {
			return a > 0 ? 1 : 0;
		}
	",
    );

    assert_eq!(scored["ternary"].cyclomatic, 2);
    assert_eq!(scored["ternary"].cognitive, 1);
}

#[test]
fn a_logical_assignment_counts_toward_cyclomatic_complexity() {
    let scored = scan(
        "
		export function defaulted(a: number | null): number {
			let value = a;

			value ??= 1;

			return value;
		}
	",
    );

    assert_eq!(scored["defaulted"].cyclomatic, 2);
}

#[test]
fn a_labelled_jump_costs_one_cognitive_point() {
    let scored = scan(
        "
		export function labelled(rows: number[][]): number {
			outer: for (const row of rows) {
				for (const cell of row) {
					if (cell > 0) {
						break outer;
					}
				}
			}

			return 0;
		}
	",
    );

    // for (+1) + for (+2) + if (+3) + labelled break (+1).
    assert_eq!(scored["labelled"].cognitive, 7);
}

#[test]
fn the_key_carries_the_file_and_the_reporting_name() {
    let scored: FxHashMap<String, ComplexityScore> =
        score("src/constructs.ts", &shared_constructs()).into_iter().map(|score| (score.name.clone(), score)).collect();

    assert_eq!(scored["ifChain"].key, "src/constructs.ts#ifChain");
    assert_eq!(scored["ifChain"].line, 2);
}

// complexity-command.test.ts (the scoring half; reading, listing, and
// scorable-path selection live in the engine)

#[test]
fn the_scan_reports_every_scored_function() {
    let scores = score("src/app.ts", "export function read(a: number): number {\n\tif (a > 0) {\n\t\treturn a;\n\t}\n\n\treturn 0;\n}\n");

    assert_eq!(scores, [ComplexityScore { key: "src/app.ts#read".to_owned(), name: "read".to_owned(), line: 1, cyclomatic: 2, cognitive: 1 }]);
}

#[test]
fn an_exported_arrow_is_one_function() {
    assert_eq!(score("src/app.ts", "export const read = (): number => 0;\n").len(), 1);
}

// line-index.test.ts (the offset-before-start case has no `u32` counterpart)

#[test]
fn offsets_resolve_to_the_one_based_line_that_contains_them() {
    let mut lines = LineCursor::new("one\ntwo\n\nfour");

    for (offset, line) in [(0, 1), (3, 1), (4, 2), (8, 3), (9, 4), (12, 4), (4, 2), (0, 1), (12, 4), (99, 4)] {
        assert_eq!(lines.line_at(offset), line, "offset {offset}");
    }
}

#[test]
fn an_empty_document_is_one_line() {
    assert_eq!(LineCursor::new("").line_at(0), 1);
}

// The fix: v1 resolved an anonymous function to its owner's *name*, so the
// callback of the second `handler` (key `handler:6`) raised `handler`.

#[test]
fn an_anonymous_child_of_a_line_keyed_function_reports_under_that_key() {
    let source = include_str!("../../../../fixtures/complexity/same-name.ts");

    assert_eq!(table("same-name.ts", source), rows(&[("handler", 2, 1, 0), ("handler:6", 6, 2, 1)]));
}

#[test]
fn a_nested_function_sharing_its_parents_name_keeps_its_own_callbacks() {
    let source = "function run() {\n\tfunction run() {\n\t\treturn [1].map((x) => x || 0);\n\t}\n}\n";

    // v1: `run` 2/1 and `run:2` 1/1, the inner callback's 2 landing on the outer key.
    assert_eq!(table("nested.ts", source), rows(&[("run", 1, 1, 1), ("run:2", 2, 2, 1)]));
}

// The fixture shared with the Go helper.

#[test]
fn the_fixtures_match_their_expected_scores() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/complexity");
    let mut checked = 0;

    for entry in std::fs::read_dir(&dir).expect("fixtures/complexity exists") {
        let path = entry.expect("a directory entry").path();

        if path.extension().is_none_or(|extension| extension != "ts") {
            continue;
        }

        let name = path.file_name().and_then(|name| name.to_str()).expect("a UTF-8 fixture name");

        let source = std::fs::read_to_string(&path).expect("a readable fixture");
        let expected = std::fs::read_to_string(dir.join(format!("{name}.json"))).expect("an expectation beside the fixture");
        let expected: serde_json::Value = serde_json::from_str(&expected).expect("valid JSON");

        let actual: Vec<serde_json::Value> = score(name, &source)
            .into_iter()
            .map(|score| serde_json::json!({ "key": score.key, "line": score.line, "cyclomatic": score.cyclomatic, "cognitive": score.cognitive }))
            .collect();

        assert_eq!(serde_json::Value::Array(actual), expected, "{name}");

        checked += 1;
    }

    assert!(checked >= 2, "only {checked} fixtures were checked");
}

// ESTree-vs-oxc shapes. Every expected row below is v1's own output for the
// same source (oxc-parser 0.152.0, ESTree JSON).

#[test]
fn names_and_scores_match_v1_across_class_and_assignment_shapes() {
    let source = "abstract class Base {
	abstract run(): void;
	accessor cb = () => (a || b);
	static Inner = class { m() { return x ? 1 : 2; } };
	#secret() {}
	[Symbol.iterator]() {}
	['lit']() {}
	[`tpl`]() {}
	1() {}
}
const p = (() => {});
module.exports.handler = function () {};
this.x = () => {};
(a as any).b = () => {};
a[0].c = () => {};
export default class { m() {} }
const o = { get v() { return 1; }, set v(x) {}, '': () => {}, [a?.b]: () => {} };
function f(cb = () => a ?? b) { return a && (b && c); }
function g() { return a && b && c || d && e; }
const x = function named() {};
declare function d(): void;
function over(a: string): void;
function over(a: any) { if (a) {} }
let q; q ||= () => {};
";

    let expected = [
        ("<anonymous>", 3, 2, 0),
        ("Base.Inner.m", 4, 2, 1),
        ("Base.#secret", 5, 1, 0),
        ("Base.Symbol.iterator", 6, 1, 0),
        ("Base.lit", 7, 1, 0),
        ("module.exports.handler", 12, 1, 0),
        ("x", 13, 1, 0),
        ("b", 14, 1, 0),
        ("a.c", 15, 1, 0),
        ("default.m", 16, 1, 0),
        ("get v", 17, 1, 0),
        ("set v", 17, 1, 0),
        ("f", 18, 3, 3),
        ("g", 19, 5, 3),
        ("named", 20, 1, 0),
        ("over", 23, 2, 1),
        ("q", 24, 1, 0),
    ];

    assert_eq!(table("edge.ts", source), rows(&expected));
}

#[test]
fn names_and_scores_match_v1_across_jsx_control_flow_and_class_bodies() {
    let source = "const o = { b() {}, B() {}, a() {}, _() {}, 'a b'() {} };
export const View = ({ items }: { items: string[] }) => <ul>{items.length > 0 && items.map((item) => <li key={item}>{item ?? '-'}</li>)}</ul>;
function chain(a?: { b?: number }) {
	if (a?.b) {
		return 1;
	} else {
		if (a) {
			return 2;
		}
	}
	try {
		return a!.b ?? 0;
	} catch (error) {
		while (true) {
			continue;
		}
	} finally {
		do {} while (a && !a);
	}
}
class Widget {
	constructor(private readonly cb = () => (a ? b : c)) {}
	static {
		const s = () => {};
	}
	method() {
		const inner = class { run() { for (const k in o) {} } };
		return { nested() { switch (1) { case 1: break; default: } } };
	}
}
const handlers = { onClick: function () { return x || y; } };
export default () => {};
";

    let expected = [
        ("_", 1, 1, 0),
        ("a", 1, 1, 0),
        ("a b", 1, 1, 0),
        ("b", 1, 1, 0),
        ("B", 1, 1, 0),
        ("View", 2, 2, 2),
        ("chain", 3, 8, 10),
        ("Widget.constructor", 22, 2, 2),
        ("s", 24, 1, 0),
        ("Widget.method", 26, 1, 4),
        ("inner.run", 27, 2, 1),
        ("nested", 28, 2, 1),
        ("onClick", 31, 2, 1),
        ("default", 32, 1, 0),
    ];

    assert_eq!(table("more.tsx", source), rows(&expected));
}

#[test]
fn names_tie_break_like_locale_compare() {
    let mut names = ["aB", "Ab", "ab", "AB", "a:7", "a", "a.b", "a b", "a_b", "a$", "a1", "a#"];

    names.sort_by(|left, right| compare(left, right));

    assert_eq!(names, ["a", "a b", "a_b", "a:7", "a.b", "a#", "a$", "a1", "ab", "aB", "Ab", "AB"]);
    assert_eq!(compare("get v", "set v"), Ordering::Less);
}
