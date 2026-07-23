import assert from 'node:assert/strict';
import { test } from 'node:test';
import { MarkdownFences } from '#sidecar/markdown-fences';

test('MarkdownFences.extractBlocks returns each fenced block with its offset', () => {
	const content = ['# Title', '', '```ts', 'const n = 1;', '```', '', 'prose', '', '~~~js', 'const m = 2;', '~~~', ''].join('\n');
	const blocks = MarkdownFences.extractBlocks(content);

	assert.equal(blocks.length, 2);

	const [first, second] = blocks;

	assert.ok(first);

	assert.ok(second);

	assert.equal(first.lang, 'ts');

	assert.equal(first.content, 'const n = 1;\n');

	assert.equal(content.slice(first.start, first.start + first.content.length), first.content);

	assert.equal(second.lang, 'js');

	assert.equal(second.content, 'const m = 2;\n');

	assert.equal(content.slice(second.start, second.start + second.content.length), second.content);
});

test('MarkdownFences.extractBlocks reads the first info-string token as the language', () => {
	const content = ['```tsx title="Example.tsx" {1,3}', 'const x = 1;', '```', ''].join('\n');
	const blocks = MarkdownFences.extractBlocks(content);

	assert.equal(blocks.length, 1);

	assert.equal(blocks[0]?.lang, 'tsx');

	assert.equal(blocks[0]?.content, 'const x = 1;\n');
});

test('MarkdownFences.extractBlocks handles indented fences and preserves body bytes', () => {
	const content = ['- item', '', '  ```ts', '  const x = 1;', '  ```', ''].join('\n');
	const blocks = MarkdownFences.extractBlocks(content);

	assert.equal(blocks.length, 1);

	assert.equal(blocks[0]?.content, '  const x = 1;\n');

	const start = blocks[0]?.start ?? 0;

	assert.equal(content.slice(start, start + (blocks[0]?.content.length ?? 0)), blocks[0]?.content);
});

test('MarkdownFences.extractBlocks requires the closing fence to be at least as long', () => {
	const content = ['````ts', 'const inner = "```";', '````', ''].join('\n');
	const blocks = MarkdownFences.extractBlocks(content);

	assert.equal(blocks.length, 1);

	assert.equal(blocks[0]?.content, 'const inner = "```";\n');
});

test('MarkdownFences.extractBlocks ignores an unterminated fence', () => {
	const content = ['```ts', 'const x = 1;', 'const y = 2;', ''].join('\n');

	assert.deepEqual(MarkdownFences.extractBlocks(content), []);
});

test('MarkdownFences.extractBlocks does not treat four-space indented code as a fence', () => {
	const content = ['    ```ts', '    const x = 1;', '    ```', ''].join('\n');

	assert.deepEqual(MarkdownFences.extractBlocks(content), []);
});

test('MarkdownFences.extractBlocks yields an empty body for an immediately closed fence', () => {
	const content = ['```ts', '```', ''].join('\n');
	const blocks = MarkdownFences.extractBlocks(content);

	assert.equal(blocks.length, 1);

	assert.equal(blocks[0]?.content, '');
});

test('MarkdownFences.extractBlocks tolerates carriage returns', () => {
	const content = ['```ts', 'const x = 1;', '```', ''].join('\r\n');
	const blocks = MarkdownFences.extractBlocks(content);

	assert.equal(blocks.length, 1);

	assert.equal(blocks[0]?.content, 'const x = 1;\r\n');

	const start = blocks[0]?.start ?? 0;

	assert.equal(content.slice(start, start + (blocks[0]?.content.length ?? 0)), blocks[0]?.content);
});

test('MarkdownFences.isJavaScriptOrTypeScript accepts JS/TS langs case-insensitively', () => {
	for (const lang of ['ts', 'TS', 'tsx', 'js', 'JSX', 'typescript', 'javascript', 'mjs', 'cjs', 'mts', 'cts']) {
		assert.equal(MarkdownFences.isJavaScriptOrTypeScript(lang), true, lang);
	}

	for (const lang of ['json', 'bash', 'sh', 'yaml', 'html', '']) {
		assert.equal(MarkdownFences.isJavaScriptOrTypeScript(lang), false, lang);
	}
});

test('MarkdownFences.scriptExtension maps JSX flavours to tsx', () => {
	assert.equal(MarkdownFences.scriptExtension('tsx'), 'tsx');

	assert.equal(MarkdownFences.scriptExtension('JSX'), 'tsx');

	assert.equal(MarkdownFences.scriptExtension('ts'), 'ts');

	assert.equal(MarkdownFences.scriptExtension('js'), 'ts');

	assert.equal(MarkdownFences.scriptExtension('typescript'), 'ts');
});
