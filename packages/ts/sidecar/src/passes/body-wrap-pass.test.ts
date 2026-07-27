import assert from 'node:assert/strict';
import { test } from 'node:test';
import { AstReader } from '#sidecar/syntax/ast-reader';
import { BodyWrapPass } from '#sidecar/passes/body-wrap-pass';
import { EditApplier } from '#sidecar/syntax/edits';
import { SourceDocument } from '#sidecar/syntax/source-document';
import { SourceParser } from '#sidecar/syntax/source-parser';

const pass = new BodyWrapPass({ parser: new SourceParser(), ast: new AstReader() });
const editApplier = new EditApplier();

function computeEdits(source: string) {
	return pass.computeEdits(SourceDocument.of('sample.ts', source));
}

function wrapOnce(source: string): string {
	const edits = computeEdits(source);

	return edits.length > 0 ? editApplier.apply(source, edits) : source;
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

test('wraps with four spaces when the source is space-indented', () => {
	const source = ['function run() {', '    if (ready) go();', '}', ''].join('\n');
	const expected = ['function run() {', '    if (ready) {', '        go();', '    }', '}', ''].join('\n');
	const result = wrapFully(source);

	assert.equal(result, expected);
	assert.ok(!result.includes('\t'), 'space-indented body wrap must not introduce tabs');
});

test('leaves already-braced bodies alone', () => {
	const source = 'if (ready) {\n\trun();\n}\n';

	assert.deepEqual(computeEdits(source), []);
});

test('returns no edits for source with syntax errors', () => {
	assert.deepEqual(computeEdits('if (broken run();\n'), []);
});
