import assert from 'node:assert/strict';
import { test } from 'node:test';
import { EmbeddedBlocks } from '#sidecar/hosts/embedded-blocks';

test('EmbeddedBlocks.isHost accepts every host extension and rejects others', () => {
	for (const path of ['a.vue', 'b.html', 'c.htm', 'd.md', 'e.markdown']) {
		assert.equal(EmbeddedBlocks.isHost(path), true, path);
	}

	for (const path of ['a.ts', 'b.tsx', 'c.json', 'd.css']) {
		assert.equal(EmbeddedBlocks.isHost(path), false, path);
	}
});

test('EmbeddedBlocks.hardValidated is true for markup and false for markdown', () => {
	assert.equal(EmbeddedBlocks.hardValidated('a.vue'), true);

	assert.equal(EmbeddedBlocks.hardValidated('b.html'), true);

	assert.equal(EmbeddedBlocks.hardValidated('c.htm'), true);

	assert.equal(EmbeddedBlocks.hardValidated('d.md'), false);

	assert.equal(EmbeddedBlocks.hardValidated('e.markdown'), false);
});

test('EmbeddedBlocks.extract reads JS/TS script blocks from Vue and HTML', () => {
	const vue = '<script lang="yaml">\nfoo: 1\n</script>\n<script setup lang="tsx">\nconst n = 1;\n</script>\n';
	const vueBlocks = EmbeddedBlocks.extract('component.vue', vue);

	assert.equal(vueBlocks.length, 1);

	assert.equal(vueBlocks[0]?.content, '\nconst n = 1;\n');

	assert.equal(vueBlocks[0]?.extension, 'tsx');

	assert.equal(vue.slice(vueBlocks[0]?.start, (vueBlocks[0]?.start ?? 0) + (vueBlocks[0]?.content.length ?? 0)), vueBlocks[0]?.content);

	const html = '<html>\n<body>\n<script>\nconst x = 1;\n</script>\n</body>\n</html>\n';
	const htmlBlocks = EmbeddedBlocks.extract('page.html', html);

	assert.equal(htmlBlocks.length, 1);

	assert.equal(htmlBlocks[0]?.extension, 'ts');

	assert.equal(htmlBlocks[0]?.content, '\nconst x = 1;\n');
});

test('EmbeddedBlocks.extract reads JS/TS fences from Markdown and skips others', () => {
	const markdown = ['```bash', 'echo hi', '```', '', '```tsx', 'const n = 1;', '```', ''].join('\n');
	const blocks = EmbeddedBlocks.extract('notes.md', markdown);

	assert.equal(blocks.length, 1);

	assert.equal(blocks[0]?.content, 'const n = 1;\n');

	assert.equal(blocks[0]?.extension, 'tsx');

	assert.equal(markdown.slice(blocks[0]?.start, (blocks[0]?.start ?? 0) + (blocks[0]?.content.length ?? 0)), blocks[0]?.content);
});

test('EmbeddedBlocks.extract returns nothing for non-host paths', () => {
	assert.deepEqual(EmbeddedBlocks.extract('app.ts', 'const x = 1;\n'), []);
});

test('EmbeddedBlocks.rewrite applies the transform per block and preserves surrounding bytes', () => {
	const markdown = ['# Title', '', '```ts', 'const a = 1;', '```', '', '```ts', 'const b = 2;', '```', ''].join('\n');
	const seen: string[] = [];

	const rewritten = EmbeddedBlocks.rewrite('notes.md', markdown, (blockContent, virtualName) => {
		seen.push(virtualName);

		return blockContent.toUpperCase();
	});

	assert.deepEqual(seen, ['notes.md.script.ts', 'notes.md.script.ts']);

	assert.ok(rewritten.startsWith('# Title\n\n```ts\n'));

	assert.ok(rewritten.includes('CONST A = 1;'));

	assert.ok(rewritten.includes('CONST B = 2;'));

	assert.ok(rewritten.includes('```'));
});

test('EmbeddedBlocks.rewrite leaves content unchanged when the transform is identity', () => {
	const html = '<script>\nconst x = 1;\n</script>\n';

	assert.equal(
		EmbeddedBlocks.rewrite('page.html', html, (blockContent) => {
			return blockContent;
		}),
		html,
	);
});
