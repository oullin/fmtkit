import assert from 'node:assert/strict';
import { test } from 'node:test';
import { VueScript } from '#sidecar/vue-script';

test('VueScript.extractBlocks returns every script block with its offset', () => {
	const content = '<script lang="yaml">\nfoo: 1\n</script>\n<script setup lang="ts">\nconst n = 1;\n</script>\n';
	const blocks = VueScript.extractBlocks(content);

	assert.equal(blocks.length, 2);

	const [first, second] = blocks;

	assert.ok(first);

	assert.ok(second);

	assert.equal(first.openTag, '<script lang="yaml">');

	assert.equal(content.slice(second.start, second.start + second.content.length), second.content);
});

test('VueScript.attribute reads quoted and bare attribute values case-insensitively', () => {
	assert.equal(VueScript.attribute('<script LANG="TS">', 'lang'), 'ts');

	assert.equal(VueScript.attribute("<script lang='tsx'>", 'lang'), 'tsx');

	assert.equal(VueScript.attribute('<script lang=jsx>', 'lang'), 'jsx');

	assert.equal(VueScript.attribute('<script setup>', 'lang'), null);
});

test('VueScript.isJavaScriptOrTypeScript accepts JS/TS langs and module types, rejects others', () => {
	assert.equal(VueScript.isJavaScriptOrTypeScript('<script setup>'), true);

	assert.equal(VueScript.isJavaScriptOrTypeScript('<script lang="ts">'), true);

	assert.equal(VueScript.isJavaScriptOrTypeScript('<script lang="yaml">'), false);

	assert.equal(VueScript.isJavaScriptOrTypeScript('<script type="module">'), true);

	assert.equal(VueScript.isJavaScriptOrTypeScript('<script type="application/ld+json">'), false);
});
