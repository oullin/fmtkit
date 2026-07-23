import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { AstReader } from '#sidecar/syntax/ast-reader';
import { EditApplier } from '#sidecar/syntax/edits';
import { ExpandedCallPass } from '#sidecar/passes/expanded-call-pass';
import { SourceDocument } from '#sidecar/syntax/source-document';
import { SourceParser } from '#sidecar/syntax/source-parser';

const editApplier = new EditApplier();
const pass = new ExpandedCallPass({ parser: new SourceParser(), ast: new AstReader(), edits: editApplier });

function format(input: string, virtualName: string): string {
	const edits = pass.computeEdits(SourceDocument.of(virtualName, input));

	return edits.length > 0 ? editApplier.apply(input, edits) : input;
}

describe('expanded call formatter', () => {
	it('expands a returned cast wrapper around a nested call argument', () => {
		const input = ['export function createAuth() {', '\treturn betterAuth(buildAuthConfig(env, db, mailer, app, options)) as unknown as SasuAuth;', '}', ''].join('\n');
		const expected = ['export function createAuth() {', '\treturn betterAuth(', '\t\tbuildAuthConfig(env, db, mailer, app, options),', '\t) as unknown as SasuAuth;', '}', ''].join('\n');

		assert.equal(format(input, 'fixture.ts'), expected);
	});

	it('expands a baseline-indented call one unit past its baseline', () => {
		// An embedded block indented below column zero: the 3-tab baseline is one
		// nesting level, so arguments land at 4 tabs and the closing paren at 3,
		// not 6 and 3.
		const input = ['\t\t\tconst value = resolveConfig(prefix, buildOptions(env), { strict: true });', ''].join('\n');
		const expected = ['\t\t\tconst value = resolveConfig(', '\t\t\t\tprefix,', '\t\t\t\tbuildOptions(env),', '\t\t\t\t{ strict: true },', '\t\t\t);', ''].join('\n');
		const output = format(input, 'fixture.ts');

		assert.equal(output, expected);

		assert.ok(!output.includes('\t\t\t\t\t'), 'arguments must be base plus one unit, never doubled');
	});

	it('expands multi-argument calls when one argument is complex', () => {
		const input = ['const value = resolveConfig(prefix, buildOptions(env), { strict: true });', ''].join('\n');
		const expected = ['const value = resolveConfig(', '\tprefix,', '\tbuildOptions(env),', '\t{ strict: true },', ');', ''].join('\n');

		assert.equal(format(input, 'fixture.ts'), expected);
	});

	it('formats nested complex calls in one stable run', () => {
		const input = ['function run() {', '\treturn outer(inner(deep(a, b)), tail);', '}', ''].join('\n');
		const expected = ['function run() {', '\treturn outer(', '\t\tinner(', '\t\t\tdeep(a, b),', '\t\t),', '\t\ttail,', '\t);', '}', ''].join('\n');
		const once = format(input, 'fixture.ts');
		const twice = format(once, 'fixture.ts');

		assert.equal(once, expected);
		assert.equal(twice, expected);
	});

	it('expands object and array arguments without rewriting their internals', () => {
		const input = ['const config = createConfig({ hooks: [init(), done] }, [first(), second]);', ''].join('\n');
		const expected = ['const config = createConfig(', '\t{ hooks: [init(), done] },', '\t[first(), second],', ');', ''].join('\n');

		assert.equal(format(input, 'fixture.ts'), expected);
	});

	it('skips calls with comments inside the argument list', () => {
		const input = ['const value = createConfig(/* keep inline */ buildOptions(env));', ''].join('\n');

		assert.equal(format(input, 'fixture.ts'), input);
	});

	it('does not add a trailing comma after a final spread argument', () => {
		const input = ['const result = invoke(buildOptions(env), ...args);', ''].join('\n');
		const expected = ['const result = invoke(', '\tbuildOptions(env),', '\t...args', ');', ''].join('\n');

		assert.equal(format(input, 'fixture.ts'), expected);
	});

	it('leaves simple transform chains and computed callbacks unchanged', () => {
		const input = ['const normalized = value.trim().toLowerCase();', 'const item = computed(() => value);', ''].join('\n');

		assert.equal(format(input, 'fixture.ts'), input);
	});

	it('leaves method-chain arguments for fluent formatters', () => {
		const input = ['const rows = builder.where(and(eq(users.id, id), eq(users.active, true)));', ''].join('\n');

		assert.equal(format(input, 'fixture.ts'), input);
	});

	it('skips declaration files', () => {
		const input = ['declare const value: ReturnType<typeof createConfig>;', ''].join('\n');

		assert.equal(format(input, 'types.d.ts'), input);
	});

	it('nests with four spaces when the source is space-indented', () => {
		const input = ['function run() {', '    return outer(inner(deep(a, b)), tail);', '}', ''].join('\n');
		const expected = ['function run() {', '    return outer(', '        inner(', '            deep(a, b),', '        ),', '        tail,', '    );', '}', ''].join('\n');
		const once = format(input, 'fixture.ts');

		assert.equal(once, expected);
		assert.equal(format(once, 'fixture.ts'), expected);
		assert.ok(!once.includes('\t'), 'expanded output must not introduce tabs into a space-indented file');
	});

	it('nests a space-indented top-level call one level deep with spaces', () => {
		const input = ['export default defineConfig({', '    cacheDir: "../../storage/.cache",', '    test: { globals: true },', '});', ''].join('\n');
		const output = format(input, 'fixture.ts');

		assert.ok(!output.includes('\t'), 'space-indented expansion must not introduce tabs');
		assert.match(output, /defineConfig\(\n {4}\{/);
	});

	it('re-indents a multiline object argument to its new depth', () => {
		// The object's inner lines were written against the statement's indent.
		// Expanding pushes the object one level deeper, so they have to move with
		// it — oxfmt used to hide this by collapsing the call straight back.
		const input = ['function send() {', '\tconst response = fetch(url, {', '\t\t...init,', '\t\theaders,', '\t});', '}', ''].join('\n');
		const expected = ['function send() {', '\tconst response = fetch(', '\t\turl,', '\t\t{', '\t\t\t...init,', '\t\t\theaders,', '\t\t},', '\t);', '}', ''].join('\n');
		const once = format(input, 'fixture.ts');

		assert.equal(once, expected);

		// The rebase reads its origin off the argument's own line, so an argument
		// already sitting at its target depth is left where it is.
		assert.equal(format(once, 'fixture.ts'), expected);
	});

	it('never re-indents the interior of a multiline template literal', () => {
		// The literal's leading whitespace is string content. Re-indenting it made
		// every pass shift the value one unit further right (issue #59).
		const input = ['const Harness = defineComponent({', '    template: `', '        <div>', '            <span>hello</span>', '        </div>', '    `,', '});', ''].join('\n');

		const expected = ['const Harness = defineComponent(', '    {', '        template: `', '        <div>', '            <span>hello</span>', '        </div>', '    `,', '    },', ');', ''].join(
			'\n',
		);

		const once = format(input, 'fixture.ts');

		assert.equal(once, expected);

		assert.equal(format(once, 'fixture.ts'), expected);
	});

	it('preserves whitespace-only lines inside a template literal', () => {
		const input = ['const query = run(build(), {', '\tsql: `', '\t\tselect 1', '   ', '\t\tfrom t', '\t`,', '});', ''].join('\n');
		const output = format(input, 'fixture.ts');

		assert.ok(output.includes('\n   \n'), 'a blank line carrying string content must keep its bytes');

		assert.equal(format(output, 'fixture.ts'), output);
	});

	/** The source between the first and last backtick — a template's literal bytes. */
	const templateBody = (source: string): string => {
		return source.slice(source.indexOf('`') + 1, source.lastIndexOf('`'));
	};

	it('reaches a fixed point for a tab-indented template literal', () => {
		const input = ['const Harness = defineComponent({', '\ttemplate: `', '\t\t<div>', '\t\t\t<span>hello</span>', '\t\t</div>', '\t`,', '});', ''].join('\n');
		const once = format(input, 'fixture.ts');

		assert.notEqual(once, input, 'the call should have expanded');

		assert.equal(format(once, 'fixture.ts'), once);

		assert.equal(templateBody(once), templateBody(input), 'template interior bytes must be preserved');
	});

	it('reaches a fixed point for a template literal with multiline interpolations', () => {
		const input = [
			'const q = build({',
			'    query: `',
			'        SELECT *',
			'        FROM ${',
			'            resolveTable(schema)',
			'        }',
			`        WHERE id = \${id}`,
			'    `,',
			'});',
			'',
		].join('\n');

		const once = format(input, 'fixture.ts');

		assert.notEqual(once, input, 'the call should have expanded');

		assert.equal(format(once, 'fixture.ts'), once);

		assert.equal(templateBody(once), templateBody(input), 'template interior bytes must be preserved');
	});

	it('reaches a fixed point for a tagged template literal', () => {
		const input = ['const styled = create({', '    styles: css`', '        color: red;', `        margin: \${spacing}px;`, '    `,', '});', ''].join('\n');
		const once = format(input, 'fixture.ts');

		assert.notEqual(once, input, 'the call should have expanded');

		assert.equal(format(once, 'fixture.ts'), once);

		assert.equal(templateBody(once), templateBody(input), 'template interior bytes must be preserved');
	});

	it('converges over a template literal nested two expansions deep', () => {
		// The pass lifts one nesting level per run, so the inner call expands on a
		// later pass than the outer one. Convergence is what matters: once every
		// level has expanded, further passes are a no-op and the literal is intact.
		const input = ['const app = createApp({', '    root: defineComponent({', '        template: `', '            <div>deep</div>', '        `,', '    }),', '});', ''].join('\n');

		let current = input;

		for (let i = 0; i < 5; i++) {
			const next = format(current, 'fixture.ts');

			if (next === current) {
				break;
			}

			current = next;
		}

		assert.notEqual(current, input, 'both calls should have expanded');
		assert.match(current, /root: defineComponent\(\n/, 'the inner call must also expand');

		assert.equal(format(current, 'fixture.ts'), current, 'the fixed point must be stable');

		assert.equal(templateBody(current), templateBody(input), 'template interior bytes must be preserved');
	});
});
