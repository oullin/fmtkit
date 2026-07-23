import assert from 'node:assert/strict';
import { test } from 'node:test';
import { isErr } from '#sidecar/kernel/result';
import { Sources } from '#sidecar/syntax/sources';

test('Sources.parse returns the program and comments for valid source', () => {
	const parsed = Sources.parse('sample.ts', 'const one = 1; // note\n');

	assert.equal(isErr(parsed), false);

	if (isErr(parsed)) {
		return;
	}

	assert.equal(parsed.value.program.type, 'Program');

	assert.equal(parsed.value.comments.length, 1);
});

test('Sources.parse carries parser errors for source with syntax errors', () => {
	const parsed = Sources.parse('sample.ts', 'const broken = {;\n');

	assert.equal(isErr(parsed), true);

	assert.ok(isErr(parsed) && parsed.error.errors.length > 0);
});
