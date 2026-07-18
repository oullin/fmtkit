import assert from 'node:assert/strict';
import { mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { SourceFileUnreadable } from '#sidecar/errors';

import { extractVueScripts, isDeclarationFile, isJavaScriptOrTypeScript, isTargetFile, lineIndent, nonOverlappingEdits, scriptAttribute } from '#sidecar/pass-utils';
import { isErr } from '#sidecar/result';
import { NodeSourceFiles } from '#sidecar/source-files';
import { Sources } from '#sidecar/sources';

test('Sources.parse returns the program and comments for valid source', () => {
	const parsed = Sources.parse('sample.ts', 'const one = 1; // note\n');

	assert.equal(isErr(parsed), false);

	if (isErr(parsed)) {
		return;
	}

	assert.equal(parsed.value.program.type, 'Program');

	assert.equal(parsed.value.comments.length, 1);
});

test('Sources.parse carries parser errors for source with syntax errors', () => {
	const parsed = Sources.parse('sample.ts', 'const broken = {;\n');

	assert.equal(isErr(parsed), true);

	assert.ok(isErr(parsed) && parsed.error.errors.length > 0);
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

test('SourceFileUnreadable identifies ENOENT-shaped causes only', () => {
	assert.equal(new SourceFileUnreadable('missing.ts', { code: 'ENOENT' }).isNotFound(), true);

	assert.equal(new SourceFileUnreadable('denied.ts', { code: 'EACCES' }).isNotFound(), false);

	assert.equal(new SourceFileUnreadable('failed.ts', new Error('nope')).isNotFound(), false);
});

test('lineIndent returns the leading whitespace of the position line', () => {
	const source = 'if (x) {\n\t\tcall();\n}\n';

	assert.equal(lineIndent(source, source.indexOf('call')), '\t\t');

	assert.equal(lineIndent(source, 0), '');
});

test('nonOverlappingEdits drops overlapping edits and sorts by start', () => {
	const kept = nonOverlappingEdits(
		[
			{ start: 10, end: 20, replacement: 'b' },
			{ start: 0, end: 5, replacement: 'a' },
			{ start: 15, end: 25, replacement: 'c' },
		],
	);

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

	const [first, second] = blocks;

	assert.ok(first);

	assert.ok(second);

	assert.equal(first.openTag, '<script lang="yaml">');

	assert.equal(content.slice(second.start, second.start + second.content.length), second.content);
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

test('NodeSourceFiles atomically replaces content and leaves no temp files', async () => {
	const dir = await mkdtemp(
		join(
			tmpdir(),
			'fmtkit-sidecar-atomic-',
		),
	);

	try {
		const file = join(dir, 'target.ts');

		await writeFile(file, 'before\n');

		const written = await new NodeSourceFiles().writeTextAtomic(file, 'after\n');

		assert.equal(isErr(written), false);

		assert.equal(await readFile(file, 'utf8'), 'after\n');

		assert.deepEqual(await readdir(dir), ['target.ts']);
	} finally {
		await rm(
			dir,
			{ recursive: true, force: true },
		);
	}
});
