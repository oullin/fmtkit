import assert from 'node:assert/strict';
import { test } from 'node:test';
import { computeBodyWrapEdits } from '#devx/body-wrapper';
import { applyEdits } from '#devx/edits';

function wrapOnce(source: string): string {
	const edits = computeBodyWrapEdits(source, 'sample.ts');

	return edits.length > 0 ? applyEdits(source, edits) : source;
}

function wrapFully(source: string): string {
	let current = source;

	for (let i = 0; i < 5; i++) {
		const next = wrapOnce(current);

		if (next === current) {
			return current;
		}

		current = next;
	}

	return current;
}

test('wraps an un-braced if body in a block', () => {
	const result = wrapFully('if (ready) run();\n');

	assert.equal(result, 'if (ready) {\n\trun();\n}\n');
});

test('wraps nested un-braced bodies across iterations', () => {
	const result = wrapFully('for (const item of items) if (item) use(item);\n');

	assert.match(result, /^for \(const item of items\) \{\n\tif \(item\) \{\n/);

	assert.match(result, /use\(item\);/);
});

test('leaves else-if chains unwrapped at the chain link', () => {
	const source = ['if (a) {', '\tone();', '} else if (b) {', '\ttwo();', '}', ''].join('\n');

	assert.equal(wrapFully(source), source);
});

test('wraps the loop body while keeping its comment', () => {
	const result = wrapFully('while (busy) tick(); // spin\n');

	assert.match(result, /^while \(busy\) \{\n\ttick\(\);\n\}/);

	assert.match(result, /\/\/ spin/);
});

test('leaves already-braced bodies alone', () => {
	const source = 'if (ready) {\n\trun();\n}\n';

	assert.deepEqual(computeBodyWrapEdits(source, 'sample.ts'), []);
});

test('returns no edits for source with syntax errors', () => {
	assert.deepEqual(computeBodyWrapEdits('if (broken run();\n', 'sample.ts'), []);
});
