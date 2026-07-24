import assert from 'node:assert/strict';
import { test } from 'node:test';
import { SourceUnparsable } from '#sidecar/kernel/errors';
import { Node, ParsedSourceDto } from '#sidecar/syntax/node-schema';
import { isErr } from '#sidecar/kernel/result';
import { Sources } from '#sidecar/syntax/sources';

test('ParsedSourceDto accepts and freezes a valid parser envelope', () => {
	const parsed = ParsedSourceDto.from({
		program: { type: 'Program', start: 0, end: 0, body: [] },
		comments: [{ type: 'Line', start: 0, end: 0, value: ' note' }],
	});

	assert.equal(parsed.success, true);

	if (!parsed.success) {
		return;
	}

	assert.ok(parsed.data.program instanceof Node);

	assert.ok(parsed.data.comments[0] instanceof Node);

	assert.equal(Object.isFrozen(parsed.data), true);

	assert.equal(Object.isFrozen(parsed.data.comments), true);
});

test('ParsedSourceDto rejects malformed parser envelopes', () => {
	const malformed = [null, {}, { program: {}, comments: [] }, { program: { type: 'Program' }, comments: {} }, { program: { type: 'Program' }, comments: [{}] }];

	for (const value of malformed) {
		assert.equal(ParsedSourceDto.from(value).success, false);
	}
});

test('Sources.parse maps a rejected parser envelope to SourceUnparsable', () => {
	const originalFrom = ParsedSourceDto.from;

	ParsedSourceDto.from = (() => {
		return Node.schema.safeParse({});
	}) as never;

	try {
		const parsed = Sources.parse('fixture.ts', 'const value = 1;\n');

		assert.ok(isErr(parsed));

		assert.ok(isErr(parsed) && parsed.error instanceof SourceUnparsable);

		assert.deepEqual(isErr(parsed) ? parsed.error.errors : [], []);
	} finally {
		ParsedSourceDto.from = originalFrom;
	}
});
