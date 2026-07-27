import assert from 'node:assert/strict';
import { test } from 'node:test';
import { EmbeddedBlockSplitter } from '#sidecar/hosts/embedded-block-splitter';
import { FileTargetPolicy } from '#sidecar/hosts/file-target-policy';
import { MarkdownFences } from '#sidecar/hosts/markdown-fences';
import { VueScript } from '#sidecar/hosts/vue-script';

const targets = new FileTargetPolicy({
	embeddedBlocks: new EmbeddedBlockSplitter({ vueScript: new VueScript(), markdownFences: new MarkdownFences() }),
});

test('isTargetFile accepts ts and host documents but not declarations', () => {
	assert.equal(targets.isTargetFile('app.ts'), true);

	assert.equal(targets.isTargetFile('widget.vue'), true);

	assert.equal(targets.isTargetFile('page.html'), true);

	assert.equal(targets.isTargetFile('page.htm'), true);

	assert.equal(targets.isTargetFile('notes.md'), true);

	assert.equal(targets.isTargetFile('notes.markdown'), true);

	assert.equal(targets.isTargetFile('types.d.ts'), false);

	assert.equal(targets.isTargetFile('data.json'), false);
});

test('isSyntaxTarget accepts every ts file plus host documents', () => {
	assert.equal(targets.isSyntaxTarget('app.ts'), true);

	assert.equal(targets.isSyntaxTarget('types.d.ts'), true);

	assert.equal(targets.isSyntaxTarget('widget.vue'), true);

	assert.equal(targets.isSyntaxTarget('page.html'), true);

	assert.equal(targets.isSyntaxTarget('notes.md'), true);

	assert.equal(targets.isSyntaxTarget('data.json'), false);
});

test('isDeclarationFile only matches .d.ts', () => {
	assert.equal(targets.isDeclarationFile('types.d.ts'), true);

	assert.equal(targets.isDeclarationFile('app.ts'), false);
});
