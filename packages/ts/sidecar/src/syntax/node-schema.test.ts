import assert from 'node:assert/strict';
import { test } from 'node:test';
import { SourceUnparsable } from '#sidecar/kernel/errors';
import { Node, ParsedSourceDto } from '#sidecar/syntax/node-schema';
import { isErr } from '#sidecar/kernel/result';
import { SourceParser } from '#sidecar/syntax/source-parser';

test('ParsedSourceDto accepts and freezes a valid parser envelope', () => {
	const parsed = ParsedSourceDto.from({
		comments: [{ end: 0, start: 0, type: 'Line', value: ' note' }],
		program: { body: [], end: 0, start: 0, type: 'Program' },
	});

	assert.equal(parsed.success, true);

	if (!parsed.success) {
		return;
	}

	assert.ok(Node.is(parsed.data.program));

	assert.ok(Node.is(parsed.data.comments[0]));

	assert.equal(Object.isFrozen(parsed.data), true);

	assert.equal(Object.isFrozen(parsed.data.comments), true);
});

test('ParsedSourceDto rejects malformed parser envelopes', () => {
	const malformed = [null, {}, { comments: [], program: {} }, { comments: {}, program: { type: 'Program' } }, { comments: [{}], program: { type: 'Program' } }];

	for (const value of malformed) {
		assert.equal(ParsedSourceDto.from(value).success, false);
	}
});

test('ParsedSourceDto.hasCommentBetween reports comments contained by a range', () => {
	const parsed = ParsedSourceDto.from({
		comments: [{ end: 20, start: 10, type: 'Line', value: ' note' }],
		program: { body: [], end: 30, start: 0, type: 'Program' },
	});

	assert.equal(parsed.success, true);

	if (!parsed.success) {
		return;
	}

	assert.equal(parsed.data.hasCommentBetween(5, 25), true);

	assert.equal(parsed.data.hasCommentBetween(12, 25), false);

	assert.equal(parsed.data.hasCommentBetween(5, 15), false);
});

test('SourceParser.parse maps a rejected parser envelope to SourceUnparsable', () => {
	const originalFrom = ParsedSourceDto.from;

	// SAFETY: the stub deliberately returns an envelope that mismatches the
	// DTO's signature so the failure branch runs; `never` installs it past the
	// static type for this test only, and `finally` restores the original.
	ParsedSourceDto.from = (() => {
		return Node.schema.safeParse({});
	}) as never;

	try {
		const parsed = new SourceParser().parse('fixture.ts', 'const value = 1;\n');

		assert.ok(isErr(parsed));

		assert.ok(isErr(parsed) && parsed.error.constructor === SourceUnparsable);

		assert.deepEqual(isErr(parsed) ? parsed.error.errors : [], []);
	} finally {
		ParsedSourceDto.from = originalFrom;
	}
});
