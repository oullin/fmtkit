import assert from 'node:assert/strict';
import { test } from 'node:test';
import { ClassReorder } from '#sidecar/class-reorder';
import { Edits } from '#sidecar/edits';

test('class members are reordered as properties, constructors, then methods', () => {
	const input = ['class Example {', '\trun() {}', '\tvalue = 1;', '\tconstructor() {}', '}', ''].join('\n');

	const edits = ClassReorder.computeEdits(input, 'fixture.ts');
	const output = Edits.apply(input, edits);

	assert.equal(edits.length, 1);
	assert.equal(output, ['class Example {', '\tvalue = 1;', '\tconstructor() {}', '\trun() {}', '}', ''].join('\n'));
});

test('class reorder skips members with comments between them', () => {
	const input = ['class Example {', '\trun() {}', '\t// Preserve this member grouping.', '\tvalue = 1;', '}', ''].join('\n');

	assert.deepEqual(ClassReorder.computeEdits(input, 'fixture.ts'), []);
});

test('class reorder skips already ordered and single-member classes', () => {
	assert.deepEqual(ClassReorder.computeEdits(['class Ordered {', '\tvalue = 1;', '\tconstructor() {}', '\trun() {}', '}', ''].join('\n'), 'ordered.ts'), []);

	assert.deepEqual(ClassReorder.computeEdits(['class Single {', '\trun() {}', '}', ''].join('\n'), 'single.ts'), []);
});
