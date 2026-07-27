import assert from 'node:assert/strict';
import { test } from 'node:test';
import { VueScript } from '#sidecar/hosts/vue-script';

const vueScript = new VueScript();

test('vueScript.extractBlocks returns every script block with its offset', () => {
	const content = '<script lang="yaml">\nfoo: 1\n</script>\n<script setup lang="ts">\nconst n = 1;\n</script>\n';
	const blocks = vueScript.extractBlocks(content);

	assert.equal(blocks.length, 2);

	const [first, second] = blocks;

	assert.ok(first);

	assert.ok(second);

	assert.equal(first.openTag, '<script lang="yaml">');

	assert.equal(content.slice(second.start, second.start + second.content.length), second.content);
});

test('vueScript.attribute reads quoted and bare attribute values case-insensitively', () => {
	assert.equal(vueScript.attribute('<script LANG="TS">', 'lang'), 'ts');

	assert.equal(vueScript.attribute("<script lang='tsx'>", 'lang'), 'tsx');

	assert.equal(vueScript.attribute('<script lang=jsx>', 'lang'), 'jsx');

	assert.equal(vueScript.attribute('<script setup>', 'lang'), null);
});

test('vueScript.isJavaScriptOrTypeScript accepts JS/TS langs and module types, rejects others', () => {
	assert.equal(vueScript.isJavaScriptOrTypeScript('<script setup>'), true);

	assert.equal(vueScript.isJavaScriptOrTypeScript('<script lang="ts">'), true);

	assert.equal(vueScript.isJavaScriptOrTypeScript('<script lang="yaml">'), false);

	assert.equal(vueScript.isJavaScriptOrTypeScript('<script type="module">'), true);

	assert.equal(vueScript.isJavaScriptOrTypeScript('<script type="application/ld+json">'), false);
});
