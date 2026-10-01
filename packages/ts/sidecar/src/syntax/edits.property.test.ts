import assert from 'node:assert/strict';
import { test } from 'node:test';
import fc from 'fast-check';
import { EditApplier } from '#sidecar/syntax/edits';
import type { Edit } from '#sidecar/syntax/edits';

const editApplier = new EditApplier();

const editCaseArbitrary = fc.string({ maxLength: 60, minLength: 1 }).chain((source) => {
	return fc
		.array(
			fc.record({
				length: fc.integer({ max: source.length, min: 1 }),
				replacement: fc.string({ maxLength: 12 }),
				start: fc.integer({ max: source.length - 1, min: 0 }),
			}),
			{ maxLength: 30 },
		)
		.map((rawEdits) => {
			const edits: Array<Edit> = rawEdits.map((edit) => {
				return {
					end: Math.min(source.length, edit.start + edit.length),
					replacement: edit.replacement,
					start: edit.start,
				};
			});

			return { edits, source };
		});
});

test('EditApplier.nonOverlapping returns a sorted non-overlapping input subset', () => {
	fc.assert(
		fc.property(editCaseArbitrary, ({ edits }) => {
			const accepted = editApplier.nonOverlapping(edits);

			for (let index = 0; index < accepted.length; index++) {
				const edit = accepted[index];

				assert.ok(edit && edits.includes(edit));

				if (index > 0) {
					assert.ok((accepted[index - 1]?.start ?? -1) <= (edit?.start ?? -1));
				}

				for (let following = index + 1; following < accepted.length; following++) {
					const next = accepted[following];

					assert.equal(Boolean(edit && next && editApplier.rangesOverlap(edit, next)), false);
				}
			}
		}),
		{ numRuns: 100 },
	);
});

test('EditApplier.apply matches applying accepted edits individually right-to-left', () => {
	fc.assert(
		fc.property(editCaseArbitrary, ({ edits, source }) => {
			const accepted = editApplier.nonOverlapping(edits);

			let individually = source;

			for (const edit of [...accepted].reverse()) {
				individually = individually.slice(0, edit.start) + edit.replacement + individually.slice(edit.end);
			}

			assert.equal(editApplier.apply(source, accepted), individually);
		}),
		{ numRuns: 100 },
	);
});
