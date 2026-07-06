import assert from 'node:assert/strict';
import { mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';

import {
	extractVueScripts,
	isDeclarationFile,
	isJavaScriptOrTypeScript,
	isNotFoundError,
	isTargetFile,
	lineIndent,
	nonOverlappingEdits,
	parseCleanly,
	scriptAttribute,
	writeFileAtomic,
} from '#devx/pass-utils';

test('parseCleanly returns the program and comments for valid source', () => {
	const parsed = parseCleanly('sample.ts', 'const one = 1; // note\n');

	assert.ok(parsed);

	assert.equal(parsed.program.type, 'Program');

	assert.equal(parsed.comments.length, 1);
});

test('parseCleanly returns null for source with syntax errors', () => {
	assert.equal(parseCleanly('sample.ts', 'const broken = {;\n'), null);
});

test('isTargetFile accepts ts and vue but not declarations', () => {
	assert.equal(isTargetFile('app.ts'), true);

	assert.equal(isTargetFile('widget.vue'), true);

	assert.equal(isTargetFile('types.d.ts'), false);

	assert.equal(isTargetFile('notes.md'), false);
});

test('isDeclarationFile only matches .d.ts', () => {
	assert.equal(isDeclarationFile('types.d.ts'), true);

	assert.equal(isDeclarationFile('app.ts'), false);
});

test('isNotFoundError matches ENOENT-shaped errors only', () => {
	assert.equal(isNotFoundError({ code: 'ENOENT' }), true);

	assert.equal(isNotFoundError({ code: 'EACCES' }), false);

	assert.equal(isNotFoundError(new Error('nope')), false);
});

test('lineIndent returns the leading whitespace of the position line', () => {
	const source = 'if (x) {\n\t\tcall();\n}\n';

	assert.equal(lineIndent(source, source.indexOf('call')), '\t\t');

	assert.equal(lineIndent(source, 0), '');
});

test('nonOverlappingEdits drops overlapping edits and sorts by start', () => {
	const kept = nonOverlappingEdits([
		{ start: 10, end: 20, replacement: 'b' },
		{ start: 0, end: 5, replacement: 'a' },
		{ start: 15, end: 25, replacement: 'c' },
	]);

	assert.deepEqual(
		kept.map((edit) => {
			return edit.start;
		}),
		[0, 10],
	);
});

test('extractVueScripts returns every script block with its offset', () => {
	const content = '<script lang="yaml">\nfoo: 1\n</script>\n<script setup lang="ts">\nconst n = 1;\n</script>\n';
	const blocks = extractVueScripts(content);

	assert.equal(blocks.length, 2);

	assert.equal(blocks[0].openTag, '<script lang="yaml">');

	assert.equal(content.slice(blocks[1].start, blocks[1].start + blocks[1].content.length), blocks[1].content);
});

test('scriptAttribute reads quoted and bare attribute values case-insensitively', () => {
	assert.equal(scriptAttribute('<script LANG="TS">', 'lang'), 'ts');

	assert.equal(scriptAttribute("<script lang='tsx'>", 'lang'), 'tsx');

	assert.equal(scriptAttribute('<script lang=jsx>', 'lang'), 'jsx');

	assert.equal(scriptAttribute('<script setup>', 'lang'), null);
});

test('isJavaScriptOrTypeScript accepts JS/TS langs and module types, rejects others', () => {
	assert.equal(isJavaScriptOrTypeScript('<script setup>'), true);

	assert.equal(isJavaScriptOrTypeScript('<script lang="ts">'), true);

	assert.equal(isJavaScriptOrTypeScript('<script lang="yaml">'), false);

	assert.equal(isJavaScriptOrTypeScript('<script type="module">'), true);

	assert.equal(isJavaScriptOrTypeScript('<script type="application/ld+json">'), false);
});

test('writeFileAtomic replaces the file content and leaves no temp files', async () => {
	const dir = await mkdtemp(join(tmpdir(), 'go-fmt-devx-atomic-'));

	try {
		const file = join(dir, 'target.ts');

		await writeFile(file, 'before\n');

		await writeFileAtomic(file, 'after\n');

		assert.equal(await readFile(file, 'utf8'), 'after\n');

		assert.deepEqual(await readdir(dir), ['target.ts']);
	} finally {
		await rm(dir, { recursive: true, force: true });
	}
});
