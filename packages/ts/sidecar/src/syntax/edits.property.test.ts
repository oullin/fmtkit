import assert from 'node:assert/strict';
import { test } from 'node:test';
import fc from 'fast-check';
import { Edits } from '#sidecar/syntax/edits';
import type { Edit } from '#sidecar/syntax/edits';

const editCaseArbitrary = fc.string({ minLength: 1, maxLength: 60 }).chain((source) => {
	return fc
		.array(
			fc.record({
				start: fc.integer({ min: 0, max: source.length - 1 }),
				length: fc.integer({ min: 1, max: source.length }),
				replacement: fc.string({ maxLength: 12 }),
			}),
			{ maxLength: 30 },
		)
		.map((rawEdits) => {
			const edits: Edit[] = rawEdits.map((edit) => {
				return {
					start: edit.start,
					end: Math.min(source.length, edit.start + edit.length),
					replacement: edit.replacement,
				};
			});

			return { source, edits };
		});
});

test('Edits.nonOverlapping returns a sorted non-overlapping input subset', () => {
	fc.assert(
		fc.property(editCaseArbitrary, ({ edits }) => {
			const accepted = Edits.nonOverlapping(edits);

			for (let index = 0; index < accepted.length; index++) {
				const edit = accepted[index];

				assert.ok(edit && edits.includes(edit));

				if (index > 0) {
					assert.ok((accepted[index - 1]?.start ?? -1) <= (edit?.start ?? -1));
				}

				for (let following = index + 1; following < accepted.length; following++) {
					const next = accepted[following];

					assert.equal(Boolean(edit && next && Edits.rangesOverlap(edit, next)), false);
				}
			}
		}),
		{ numRuns: 100 },
	);
});

test('Edits.apply matches applying accepted edits individually right-to-left', () => {
	fc.assert(
		fc.property(editCaseArbitrary, ({ source, edits }) => {
			const accepted = Edits.nonOverlapping(edits);

			let individually = source;

			for (const edit of [...accepted].reverse()) {
				individually = individually.slice(0, edit.start) + edit.replacement + individually.slice(edit.end);
			}

			assert.equal(Edits.apply(source, accepted), individually);
		}),
		{ numRuns: 100 },
	);
});
