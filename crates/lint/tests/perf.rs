//! The `perf/*` rules: what each reports, where, and what it leaves alone.

mod support;

use serde_json::{Value, json};
use support::{lint, only};

/// Lint `code` as `file` with `rule` alone and compare the reported
/// `(line, column)` sites, then check every message names the rule's advice.
fn assert_sites(rule: &str, options: &[Value], file: &str, code: &str, expected: &[(u32, u32)]) {
    let result = lint(&only(rule, options), file, code, false);
    let actual: Vec<_> = result.diagnostics.iter().map(|d| (d.line, d.column)).collect();

    assert_eq!(actual, expected, "{rule} on {file}:\n{code}");
    assert!(result.diagnostics.iter().all(|d| d.rule == rule), "{:?}", result.diagnostics);
}

fn messages(rule: &str, options: &[Value], code: &str) -> Vec<String> {
    lint(&only(rule, options), "a.ts", code, false).diagnostics.into_iter().map(|d| d.message).collect()
}

const REGEX: &str = "perf/no-regex-in-function";

#[test]
fn regex_in_function_reports_patterns_built_per_call() {
    let code = "\
const SHARED = /a/u;
export function f(s: string): boolean {
\treturn /b/u.test(s) || SHARED.test(s);
}
export const g = (s: string) => s.replace(/c/gu, '');
export class C {
\tm(s: string) {
\t\treturn new RegExp('d', 'u').test(s) || RegExp(`e`).test(s);
\t}
}
";

    assert_sites(REGEX, &[], "a.ts", code, &[(3, 9), (5, 43), (8, 10), (8, 42)]);
    assert_eq!(messages(REGEX, &[], "function f() { return /a/u; }"), ["This regular expression is built on every call. Hoist it to a module-level constant."]);
}

#[test]
fn regex_in_function_leaves_stateful_locals_dynamic_patterns_and_tests() {
    let code = "\
export function f(s: string, source: string): string[] {
\tconst re = /a/gu;
\tconst dynamic = new RegExp(source, 'u');
\tconst built = new RegExp(`${source}+`);
\treturn [...s.matchAll(re), dynamic, built].map(String);
}
export class C {
\tfield = /b/u;
}
";

    assert_sites(REGEX, &[], "a.ts", code, &[]);
    assert_sites(REGEX, &[], "a.test.ts", "it('x', () => { expect('a').toMatch(/a/u); });", &[]);
}

#[test]
fn regex_in_function_shares_stateful_patterns_only_where_safe() {
    let code = "\
export function f(s: string, check: (r: RegExp) => boolean) {
\tconst re = new RegExp('a', 'g');
\tconst sticky = RegExp(`b`, `y`);
\treturn [check(/c/g), s.match(/d/y), s.split(/,/y), s.match(/e/g), re, sticky];
}
";

    assert_sites(REGEX, &[], "a.ts", code, &[(4, 46), (4, 61)]);
}

const STATEFUL: &str = "perf/no-stateful-regex-test";

#[test]
fn stateful_regex_test_reports_global_and_sticky_patterns() {
    let code = "\
const RUNS = /\\s+/gu;
const STICKY = new RegExp('a', 'y');
const PLAIN = /\\s+/u;
export const a = (s: string) => RUNS.test(s);
export const b = (s: string) => STICKY.exec(s);
export const c = (s: string) => /x/g.test(s);
export const d = (s: string) => PLAIN.test(s) || s.replace(RUNS, ' ');
";

    assert_sites(STATEFUL, &[], "a.ts", code, &[(4, 33), (5, 33), (6, 33)]);
    assert_eq!(
        messages(STATEFUL, &[], "const R = /a/g;\nR.exec('a');"),
        ["`exec` on a pattern with the `g` or `y` flag starts from the previous match's `lastIndex`. Use a pattern without those flags."]
    );
}

#[test]
fn stateful_regex_test_ignores_let_bindings_and_imports() {
    let code = "\
import { RUNS } from './patterns';
let local = /a/g;
export const a = (s: string) => RUNS.test(s) || local.test(s);
";

    assert_sites(STATEFUL, &[], "a.ts", code, &[]);
}

#[test]
fn stateful_regex_test_leaves_scanners_that_position_the_pattern() {
    let code = "\
const TOKEN = /\\w+/y;
export function scan(text: string, at: number) {
\tTOKEN.lastIndex = at;
\treturn TOKEN.exec(text);
}
export const other = (text: string) => TOKEN.exec(text);
";

    assert_sites(STATEFUL, &[], "a.ts", code, &[(6, 40)]);
}

#[test]
fn stateful_regex_test_needs_the_position_set_before_the_call() {
    let code = "\
const TOKEN = /\\w+/y;
export function scan(text: string, at: number) {
\tconst match = TOKEN.exec(text);
\tTOKEN.lastIndex = at;
\treturn match;
}
";

    assert_sites(STATEFUL, &[], "a.ts", code, &[(3, 16)]);
}

const SHARED: &str = "perf/prefer-shared-pattern";

fn shared_options() -> Vec<Value> {
    vec![json!({ "module": "@primitives/patterns", "patterns": { "WHITESPACE": "/\\s+/u", "TRAILING_SLASH": "/\\/$/u" } })]
}

#[test]
fn prefer_shared_pattern_reports_repeated_literals() {
    let code = "\
export const words = (s: string) => s.split(/\\s+/u);
export const trim = (s: string) => s.replace(/\\/$/u, '');
export const other = /\\s+/gu;
";

    assert_sites(SHARED, &shared_options(), "a.ts", code, &[(1, 45), (2, 46)]);
    assert_eq!(
        messages(SHARED, &shared_options(), "const s = /\\s+/u;"),
        ["`/\\s+/u` is `WHITESPACE` in `@primitives/patterns`. Import it from there instead of writing the pattern again."]
    );
}

#[test]
fn prefer_shared_pattern_leaves_the_defining_declaration() {
    assert_sites(SHARED, &shared_options(), "patterns.ts", "export const WHITESPACE = /\\s+/u;\nexport const TRAILING_SLASH = /\\/$/u;\n", &[]);
}

#[test]
fn prefer_shared_pattern_validates_its_options() {
    let error = |options: Value| {
        fmtkit_lint::Linter::new(&fmtkit_config::Lint { bundled: false, rules: [(SHARED.to_owned(), json!(["error", options]))].into(), ignore: Vec::new() })
            .err()
            .map(|e| e.to_string())
    };

    assert!(error(json!({ "patterns": { "A": "/a/" } })).is_some_and(|e| e.contains("\"module\" is required")));
    assert!(error(json!({ "module": "m", "patterns": {} })).is_some_and(|e| e.contains("at least one pattern")));
    assert!(error(json!({ "module": "m", "patterns": { "A": "a" } })).is_some_and(|e| e.contains("must be a literal")));
    assert!(error(json!({ "module": "m", "patterns": { "A": "/abc" } })).is_some_and(|e| e.contains("must be a literal")));
    assert!(error(json!({ "module": "m", "patterns": { "A": "//" } })).is_some_and(|e| e.contains("must be a literal")));
    assert!(error(json!({ "module": "m", "patterns": { "A": "/a/", "B": "/a/" } })).is_some_and(|e| e.contains("share the literal")));
    assert!(error(json!({ "module": "m", "patterns": { "A": "/a/" }, "extra": true })).is_some_and(|e| e.contains("unknown option")));
}

const URL_PARSE: &str = "perf/no-repeated-url-parse";

#[test]
fn repeated_url_parse_reports_the_second_parse_in_a_scope() {
    let code = "\
export function f(base: string, runtime: { baseUrl: string }) {
\tconst origin = new URL(base).origin;
\tconst host = new URL(base).hostname;
\tconst a = new URL(runtime.baseUrl).origin;
\treturn [origin, host, a, new URL(runtime.baseUrl).pathname];
}
";

    assert_sites(URL_PARSE, &[], "a.ts", code, &[(3, 15), (5, 27)]);
}

#[test]
fn repeated_url_parse_keeps_scopes_and_unstable_inputs_apart() {
    let code = "\
export const a = (base: string) => new URL(base).origin;
export const b = (base: string) => new URL(base).hostname;
export function c(next: () => string) {
\treturn [new URL(next()).href, new URL(next()).href, new URL(`${'a'}`).href, new URL(`${'a'}`).href];
}
export function d(path: string, base: string) {
\treturn [new URL(path, base).href, new URL(base).href];
}
";

    assert_sites(URL_PARSE, &[], "a.ts", code, &[]);
}

#[test]
fn repeated_url_parse_keeps_bindings_with_the_same_name_apart() {
    let code = "\
export function f(first: string[], second: string[]) {
\tfor (const u of first) new URL(u).origin;
\tfor (const u of second) new URL(u).host;
}
";

    assert_sites(URL_PARSE, &[], "a.ts", code, &[]);
}

const SELECT: &str = "perf/no-select-all";

#[test]
fn select_all_reports_bare_selects_and_returning() {
    let code = "\
export async function f(db: Db) {
\tawait db.select().from(users);
\tawait db.selectDistinct().from(users);
\tawait db.insert(users).values(row).returning();
\tawait db.update(users).set(row).where(id).returning();
\tawait db.delete(users).returning();
}
";

    assert_sites(SELECT, &[], "a.ts", code, &[(2, 8), (3, 8), (4, 8), (5, 8), (6, 8)]);
    assert_eq!(
        messages(SELECT, &[], "db.select().from(users);"),
        ["`select()` reads every column. Pass only the columns the caller reads, as `select({ id: table.id })`."]
    );
}

#[test]
fn select_all_leaves_column_maps_and_unrelated_methods() {
    let code = "\
export async function f(db: Db, el: HTMLInputElement, query: Query) {
\tawait db.select({ id: users.id }).from(users);
\tawait db.insert(users).values(row).returning({ id: users.id });
\tel.select();
\tquery.returning();
}
";

    assert_sites(SELECT, &[], "a.ts", code, &[]);
}

const LOOP: &str = "perf/no-query-in-loop";

#[test]
fn query_in_loop_reports_awaited_queries_in_loop_bodies() {
    let code = "\
export class R {
\tconstructor(private readonly db: Db) {}
\tasync f(ids: string[], tx: Tx) {
\t\tfor (const id of ids) {
\t\t\tawait this.db.select({ id: t.id }).from(t).where(eq(t.id, id));
\t\t}
\t\tfor (let i = 0; i < ids.length; i++) await tx.delete(t);
\t\twhile (ids.length > 0) {
\t\t\tawait tx.update(t).set({ a: ids.pop() });
\t\t}
\t}
}
";

    assert_sites(LOOP, &[], "a.ts", code, &[(5, 4), (7, 40), (9, 4)]);
    assert_eq!(
        messages(LOOP, &[], "async function f(ids: string[]) { for (const id of ids) { await db.delete(t); } }"),
        ["`db` is queried once per loop iteration. Read every item in one query (`inArray`, a join, or one `sql` statement) before the loop."]
    );
}

#[test]
fn query_in_loop_leaves_iterables_callbacks_and_other_receivers() {
    let code = "\
export async function f(ids: string[]) {
\tfor (const row of await db.select({ id: t.id }).from(t)) {
\t\tawait client.fetch(row.id);
\t}
\tawait Promise.all(ids.map(async (id) => db.delete(t).where(eq(t.id, id))));
\tfor (const id of ids) {
\t\tids.forEach(async () => { await db.delete(t); });
\t}
}
";

    assert_sites(LOOP, &[], "a.ts", code, &[]);
}

#[test]
fn query_in_loop_receivers_replace_the_defaults() {
    let code = "async function f(ids: string[]) {\n\tfor (const id of ids) {\n\t\tawait db.delete(t);\n\t\tawait repo.find(id);\n\t}\n}\n";

    assert_sites(LOOP, &[json!({ "receivers": ["repo"] })], "a.ts", code, &[(4, 3)]);
}
