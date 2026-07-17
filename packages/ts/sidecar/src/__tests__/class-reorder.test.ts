import assert from 'node:assert/strict';
import { test } from 'node:test';
import { computeReorderEdits } from '#devx/class-reorder';
import { applyEdits } from '#devx/edits';

test('class members are reordered as properties, constructors, then methods', () => {
	const input = ['class Example {', '\trun() {}', '\tvalue = 1;', '\tconstructor() {}', '}', ''].join('\n');

	const edits = computeReorderEdits(input, 'fixture.ts');
	const output = applyEdits(input, edits);

	assert.equal(edits.length, 1);
	assert.equal(output, ['class Example {', '\tvalue = 1;', '\tconstructor() {}', '\trun() {}', '}', ''].join('\n'));
});

test('class reorder skips members with comments between them', () => {
	const input = ['class Example {', '\trun() {}', '\t// Preserve this member grouping.', '\tvalue = 1;', '}', ''].join('\n');

	assert.deepEqual(computeReorderEdits(input, 'fixture.ts'), []);
});

test('class reorder skips already ordered and single-member classes', () => {
	assert.deepEqual(computeReorderEdits(['class Ordered {', '\tvalue = 1;', '\tconstructor() {}', '\trun() {}', '}', ''].join('\n'), 'ordered.ts'), []);

	assert.deepEqual(computeReorderEdits(['class Single {', '\trun() {}', '}', ''].join('\n'), 'single.ts'), []);
});
