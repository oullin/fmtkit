import assert from 'node:assert/strict';
import { test } from 'node:test';
import fc from 'fast-check';
import { MarkdownFences } from '#sidecar/hosts/markdown-fences';
import type { MarkdownFenceBlock } from '#sidecar/hosts/markdown-fences';

const markdownFences = new MarkdownFences();

type ExpectedBlock = {
	readonly content: string;
	readonly lang: string;
	readonly start: number;
};

const fenceArbitrary = fc.constantFrom('```', '````', '~~~', '~~~~');
const langArbitrary = fc.constantFrom('ts', 'tsx', 'js', 'jsx', 'typescript', 'javascript', 'mjs', 'json', 'bash', 'yaml', '');
const bodyLineArbitrary = fc.constantFrom('const value = 1;', 'export default {};', '// a comment', 'let total = sum(a, b);');
const bodyArbitrary = fc.array(bodyLineArbitrary, { maxLength: 3, minLength: 0 });

function assertBlock(document: string, block: MarkdownFenceBlock | undefined, generated: ExpectedBlock | undefined): void {
	assert.ok(block);
	assert.ok(generated);
	assert.equal(block.lang, generated.lang);
	assert.equal(block.content, generated.content);
	assert.equal(block.start, generated.start);
	assert.equal(document.slice(block.start, block.start + block.content.length), block.content);
}

const documentArbitrary = fc.array(fc.record({ body: bodyArbitrary, fence: fenceArbitrary, lang: langArbitrary }), { maxLength: 6, minLength: 1 }).map((specs) => {
	let document = '';

	const expected: Array<ExpectedBlock> = [];

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
		expected.push({ content, lang: spec.lang, start });
		document += `${spec.fence}\n\n`;
	}

	return { document, expected };
});

test('markdownFences.extractBlocks preserves generated content offsets and language detection', () => {
	fc.assert(
		fc.property(documentArbitrary, ({ document, expected }) => {
			const extracted = markdownFences.extractBlocks(document);

			assert.equal(extracted.length, expected.length);

			for (let index = 0; index < extracted.length; index++) {
				assertBlock(document, extracted[index], expected[index]);
			}
		}),
		{ numRuns: 200 },
	);
});
