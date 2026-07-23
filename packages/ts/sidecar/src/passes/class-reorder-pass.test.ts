import assert from 'node:assert/strict';
import { test } from 'node:test';
import { AstReader } from '#sidecar/syntax/ast-reader';
import { ClassMemberPolicy } from '#sidecar/passes/policies/class-member-policy';
import { ClassReorderPass } from '#sidecar/passes/class-reorder-pass';
import { EditApplier } from '#sidecar/syntax/edits';
import { SourceDocument } from '#sidecar/syntax/source-document';
import { SourceParser } from '#sidecar/syntax/source-parser';

const ast = new AstReader();
const pass = new ClassReorderPass({ parser: new SourceParser(), ast, members: new ClassMemberPolicy({ ast }) });

function computeEdits(source: string, virtualName: string) {
	return pass.computeEdits(SourceDocument.of(virtualName, source));
}

test('class members are reordered as properties, constructors, then methods', () => {
	const input = ['class Example {', '\trun() {}', '\tvalue = 1;', '\tconstructor() {}', '}', ''].join('\n');

	const edits = computeEdits(input, 'fixture.ts');
	const output = new EditApplier().apply(input, edits);

	assert.equal(edits.length, 1);
	assert.equal(output, ['class Example {', '\tvalue = 1;', '\tconstructor() {}', '\trun() {}', '}', ''].join('\n'));
});

test('class reorder skips members with comments between them', () => {
	const input = ['class Example {', '\trun() {}', '\t// Preserve this member grouping.', '\tvalue = 1;', '}', ''].join('\n');

	assert.deepEqual(computeEdits(input, 'fixture.ts'), []);
});

test('class reorder skips already ordered and single-member classes', () => {
	assert.deepEqual(computeEdits(['class Ordered {', '\tvalue = 1;', '\tconstructor() {}', '\trun() {}', '}', ''].join('\n'), 'ordered.ts'), []);

	assert.deepEqual(computeEdits(['class Single {', '\trun() {}', '}', ''].join('\n'), 'single.ts'), []);
});
