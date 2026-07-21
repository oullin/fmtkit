import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { ExpandedCalls } from '#sidecar/expanded-calls';

describe('expanded call formatter', () => {
	it('expands a returned cast wrapper around a nested call argument', () => {
		const input = ['export function createAuth() {', '\treturn betterAuth(buildAuthConfig(env, db, mailer, app, options)) as unknown as SasuAuth;', '}', ''].join('\n');
		const expected = ['export function createAuth() {', '\treturn betterAuth(', '\t\tbuildAuthConfig(env, db, mailer, app, options),', '\t) as unknown as SasuAuth;', '}', ''].join('\n');

		assert.equal(ExpandedCalls.format(input, 'fixture.ts'), expected);
	});

	it('expands multi-argument calls when one argument is complex', () => {
		const input = ['const value = resolveConfig(prefix, buildOptions(env), { strict: true });', ''].join('\n');
		const expected = ['const value = resolveConfig(', '\tprefix,', '\tbuildOptions(env),', '\t{ strict: true },', ');', ''].join('\n');

		assert.equal(ExpandedCalls.format(input, 'fixture.ts'), expected);
	});

	it('formats nested complex calls in one stable run', () => {
		const input = ['function run() {', '\treturn outer(inner(deep(a, b)), tail);', '}', ''].join('\n');
		const expected = ['function run() {', '\treturn outer(', '\t\tinner(', '\t\t\tdeep(a, b),', '\t\t),', '\t\ttail,', '\t);', '}', ''].join('\n');
		const once = ExpandedCalls.format(input, 'fixture.ts');
		const twice = ExpandedCalls.format(once, 'fixture.ts');

		assert.equal(once, expected);
		assert.equal(twice, expected);
	});

	it('expands object and array arguments without rewriting their internals', () => {
		const input = ['const config = createConfig({ hooks: [init(), done] }, [first(), second]);', ''].join('\n');
		const expected = ['const config = createConfig(', '\t{ hooks: [init(), done] },', '\t[first(), second],', ');', ''].join('\n');

		assert.equal(ExpandedCalls.format(input, 'fixture.ts'), expected);
	});

	it('skips calls with comments inside the argument list', () => {
		const input = ['const value = createConfig(/* keep inline */ buildOptions(env));', ''].join('\n');

		assert.equal(ExpandedCalls.format(input, 'fixture.ts'), input);
	});

	it('does not add a trailing comma after a final spread argument', () => {
		const input = ['const result = invoke(buildOptions(env), ...args);', ''].join('\n');
		const expected = ['const result = invoke(', '\tbuildOptions(env),', '\t...args', ');', ''].join('\n');

		assert.equal(ExpandedCalls.format(input, 'fixture.ts'), expected);
	});

	it('leaves simple transform chains and computed callbacks unchanged', () => {
		const input = ['const normalized = value.trim().toLowerCase();', 'const item = computed(() => value);', ''].join('\n');

		assert.equal(ExpandedCalls.format(input, 'fixture.ts'), input);
	});

	it('leaves method-chain arguments for fluent formatters', () => {
		const input = ['const rows = builder.where(and(eq(users.id, id), eq(users.active, true)));', ''].join('\n');

		assert.equal(ExpandedCalls.format(input, 'fixture.ts'), input);
	});

	it('skips declaration files', () => {
		const input = ['declare const value: ReturnType<typeof createConfig>;', ''].join('\n');

		assert.equal(ExpandedCalls.format(input, 'types.d.ts'), input);
	});

	it('nests with four spaces when the source is space-indented', () => {
		const input = ['function run() {', '    return outer(inner(deep(a, b)), tail);', '}', ''].join('\n');
		const expected = ['function run() {', '    return outer(', '        inner(', '            deep(a, b),', '        ),', '        tail,', '    );', '}', ''].join('\n');
		const once = ExpandedCalls.format(input, 'fixture.ts');

		assert.equal(once, expected);
		assert.equal(ExpandedCalls.format(once, 'fixture.ts'), expected);
		assert.ok(!once.includes('\t'), 'expanded output must not introduce tabs into a space-indented file');
	});

	it('nests a space-indented top-level call one level deep with spaces', () => {
		const input = ['export default defineConfig({', '    cacheDir: "../../storage/.cache",', '    test: { globals: true },', '});', ''].join('\n');
		const output = ExpandedCalls.format(input, 'fixture.ts');

		assert.ok(!output.includes('\t'), 'space-indented expansion must not introduce tabs');
		assert.match(output, /defineConfig\(\n {4}\{/);
	});

	it('re-indents a multiline object argument to its new depth', () => {
		// The object's inner lines were written against the statement's indent.
		// Expanding pushes the object one level deeper, so they have to move with
		// it — oxfmt used to hide this by collapsing the call straight back.
		const input = ['function send() {', '\tconst response = fetch(url, {', '\t\t...init,', '\t\theaders,', '\t});', '}', ''].join('\n');
		const expected = ['function send() {', '\tconst response = fetch(', '\t\turl,', '\t\t{', '\t\t\t...init,', '\t\t\theaders,', '\t\t},', '\t);', '}', ''].join('\n');

		assert.equal(ExpandedCalls.format(input, 'fixture.ts'), expected);
	});
});
