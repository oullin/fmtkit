import assert from 'node:assert/strict';
import { test } from 'node:test';
import fc from 'fast-check';
import { MarkdownFences } from '#sidecar/markdown-fences';

type ExpectedBlock = {
	readonly lang: string;
	readonly content: string;
	readonly start: number;
};

const fenceArbitrary = fc.constantFrom('```', '````', '~~~', '~~~~');
const langArbitrary = fc.constantFrom('ts', 'tsx', 'js', 'jsx', 'typescript', 'javascript', 'mjs', 'json', 'bash', 'yaml', '');
const bodyLineArbitrary = fc.constantFrom('const value = 1;', 'export default {};', '// a comment', 'let total = sum(a, b);');
const bodyArbitrary = fc.array(bodyLineArbitrary, { minLength: 0, maxLength: 3 });

const documentArbitrary = fc.array(fc.record({ fence: fenceArbitrary, lang: langArbitrary, body: bodyArbitrary }), { minLength: 1, maxLength: 6 }).map((specs) => {
	let document = '';

	const expected: ExpectedBlock[] = [];

	for (const spec of specs) {
		document += 'intro prose line\n\n';
		document += `${spec.fence}${spec.lang}\n`;

		const start = document.length;

		const content = spec.body
			.map((line) => {
				return `${line}\n`;
			})
			.join('');

		document += content;
		expected.push({ lang: spec.lang, content, start });
		document += `${spec.fence}\n\n`;
	}

	return { document, expected };
});

test('MarkdownFences.extractBlocks preserves generated content offsets and language detection', () => {
	fc.assert(
		fc.property(documentArbitrary, ({ document, expected }) => {
			const extracted = MarkdownFences.extractBlocks(document);

			assert.equal(extracted.length, expected.length);

			for (let index = 0; index < extracted.length; index++) {
				const block = extracted[index];
				const generated = expected[index];

				assert.ok(block && generated);

				assert.equal(block?.lang, generated?.lang);

				assert.equal(block?.content, generated?.content);

				assert.equal(block?.start, generated?.start);

				assert.equal(document.slice(block?.start, (block?.start ?? 0) + (block?.content.length ?? 0)), block?.content);
			}
		}),
		{ numRuns: 200 },
	);
});
