import assert from 'node:assert/strict';
import { test } from 'node:test';
import { AstReader } from '#sidecar/syntax/ast-reader';
import { BlankLinePass } from '#sidecar/passes/blank-line-pass';
import { ClassMemberPolicy } from '#sidecar/passes/policies/class-member-policy';
import { EditApplier } from '#sidecar/syntax/edits';
import { SourceDocument } from '#sidecar/syntax/source-document';
import { SourceParser } from '#sidecar/syntax/source-parser';
import { StatementSpacingPolicy } from '#sidecar/passes/policies/statement-spacing-policy';
import { VueReactivityIdioms } from '#sidecar/passes/policies/vue-reactivity-idioms';
import type { Edit } from '#sidecar/syntax/edits';

const editApplier = new EditApplier();

function makePass(): BlankLinePass {
	const ast = new AstReader();
	const members = new ClassMemberPolicy({ ast });
	const vue = new VueReactivityIdioms({ ast });
	const spacing = new StatementSpacingPolicy({ ast, members, vue });

	return new BlankLinePass({ parser: new SourceParser(), ast, spacing });
}

/**
 * A standalone blank-line insertion, kept here as the byte-for-byte reference
 * the BlankLinePass zero-width-insert adaptation must reproduce. It dedupes
 * positions, sorts them descending, and inserts one newline at each.
 */
function referenceInsert(content: string, positions: number[]): string {
	const sorted = [...new Set(positions)].sort((a, b) => {
		return b - a;
	});

	let out = content;

	for (const pos of sorted) {
		out = out.slice(0, pos) + '\n' + out.slice(pos);
	}

	return out;
}

function zeroWidthInserts(positions: number[]): Edit[] {
	return [...new Set(positions)].map((position) => {
		return { start: position, end: position, replacement: '\n' };
	});
}

test('EditApplier byte-matches the old insert for multiple distinct positions', () => {
	const source = 'alpha\nbeta\ngamma\n';
	const positions = [0, 6, 11];

	assert.equal(editApplier.apply(source, zeroWidthInserts(positions)), referenceInsert(source, positions));
});

test('EditApplier byte-matches the old insert for adjacent positions', () => {
	const source = 'abcdef';
	const positions = [3, 4];

	assert.equal(editApplier.apply(source, zeroWidthInserts(positions)), referenceInsert(source, positions));
});

test('EditApplier byte-matches the old insert at offset zero and end of file', () => {
	const source = 'abc';
	const positions = [0, source.length];

	assert.equal(editApplier.apply(source, zeroWidthInserts(positions)), referenceInsert(source, positions));
});

test('deduplicated zero-width inserts match the old insert for repeated positions', () => {
	const source = 'let value = 1; doWork(); let next = 2;\n';
	const duplicate = [15, 15, 24];

	// The old insert collapsed repeats via a Set; the pass dedupes positions the
	// same way, so a single newline lands at the shared offset.
	assert.equal(editApplier.apply(source, zeroWidthInserts(duplicate)), referenceInsert(source, duplicate));
});

test('inserts a blank line between an import and a following function', () => {
	const pass = makePass();
	const source = ['import { foo } from "node:foo";', 'export function bar() {', '\treturn foo();', '}', ''].join('\n');
	const document = SourceDocument.of('fixture.ts', source);

	const output = editApplier.apply(source, pass.computeEdits(document));

	assert.equal(output, ['import { foo } from "node:foo";', '', 'export function bar() {', '\treturn foo();', '}', ''].join('\n'));
});

test('proposes only zero-width newline inserts', () => {
	const pass = makePass();
	const source = ['import { foo } from "node:foo";', 'export function bar() {', '\treturn foo();', '}', ''].join('\n');

	const edits = pass.computeEdits(SourceDocument.of('fixture.ts', source));

	assert.ok(edits.length > 0);

	for (const edit of edits) {
		assert.equal(edit.start, edit.end);
		assert.equal(edit.replacement, '\n');
	}
});

test('leaves an already-spaced document unchanged', () => {
	const pass = makePass();
	const source = ['import { foo } from "node:foo";', '', 'export function bar() {', '\treturn foo();', '}', ''].join('\n');

	assert.deepEqual(pass.computeEdits(SourceDocument.of('fixture.ts', source)), []);
});

test('returns no edits for source with syntax errors', () => {
	const pass = makePass();

	assert.deepEqual(pass.computeEdits(SourceDocument.of('fixture.ts', 'function broken( {\n')), []);
});
