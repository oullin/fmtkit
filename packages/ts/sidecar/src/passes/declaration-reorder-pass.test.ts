import assert from 'node:assert/strict';
import { test } from 'node:test';
import { AstReader } from '#sidecar/syntax/ast-reader';
import { DeclarationReorderPass } from '#sidecar/passes/declaration-reorder-pass';
import { EditApplier } from '#sidecar/syntax/edits';
import { SourceDocument } from '#sidecar/syntax/source-document';
import { SourceParser } from '#sidecar/syntax/source-parser';

const pass = new DeclarationReorderPass({ parser: new SourceParser(), ast: new AstReader() });
const editApplier = new EditApplier();

function computeEdits(source: string) {
	return pass.computeEdits(SourceDocument.of('sample.ts', source));
}

function reorder(source: string): string {
	const edits = computeEdits(source);

	return edits.length > 0 ? editApplier.apply(source, edits) : source;
}

test('moves single-line consts ahead of a multiline const when initializers are side-effect free', () => {
	const source = ['const big = {', '\tone: 1,', '\ttwo: 2,', '};', 'const small = 1;', ''].join('\n');

	const result = reorder(source);

	assert.match(result, /^const small = 1;\n\nconst big = \{/);
});

test('keeps order when a const initializer can have side effects', () => {
	const source = ['const big = load({', '\tone: 1,', '});', 'const small = 1;', ''].join('\n');

	const result = reorder(source);

	assert.equal(result.indexOf('const big'), source.indexOf('const big'));

	assert.match(result, /const big = load\(\{[\s\S]*const small = 1;/);
});

test('keeps order when a later single-line const uses a multiline const', () => {
	const source = ['const big = {', '\tone: 1,', '};', 'const small = big;', ''].join('\n');

	const result = reorder(source);

	assert.match(result, /const big = \{[\s\S]*const small = big;/);
});

test('reorders import groups so single-line imports precede multiline ones', () => {
	const source = ['import {', '\talpha,', '\tbeta,', "} from 'wide';", "import { tiny } from 'narrow';", ''].join('\n');

	const result = reorder(source);

	assert.match(result, /^import \{ tiny \} from 'narrow';\n\nimport \{/);
});

test('returns no edits for source with syntax errors', () => {
	assert.deepEqual(computeEdits('const broken = {;\nconst small = 1;\n'), []);
});
