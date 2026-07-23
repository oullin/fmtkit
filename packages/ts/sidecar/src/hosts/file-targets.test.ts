import assert from 'node:assert/strict';
import { test } from 'node:test';
import { FileTargets } from '#sidecar/hosts/file-targets';

test('isTargetFile accepts ts and host documents but not declarations', () => {
	assert.equal(FileTargets.isTargetFile('app.ts'), true);

	assert.equal(FileTargets.isTargetFile('widget.vue'), true);

	assert.equal(FileTargets.isTargetFile('page.html'), true);

	assert.equal(FileTargets.isTargetFile('page.htm'), true);

	assert.equal(FileTargets.isTargetFile('notes.md'), true);

	assert.equal(FileTargets.isTargetFile('notes.markdown'), true);

	assert.equal(FileTargets.isTargetFile('types.d.ts'), false);

	assert.equal(FileTargets.isTargetFile('data.json'), false);
});

test('isSyntaxTarget accepts every ts file plus host documents', () => {
	assert.equal(FileTargets.isSyntaxTarget('app.ts'), true);

	assert.equal(FileTargets.isSyntaxTarget('types.d.ts'), true);

	assert.equal(FileTargets.isSyntaxTarget('widget.vue'), true);

	assert.equal(FileTargets.isSyntaxTarget('page.html'), true);

	assert.equal(FileTargets.isSyntaxTarget('notes.md'), true);

	assert.equal(FileTargets.isSyntaxTarget('data.json'), false);
});

test('isDeclarationFile only matches .d.ts', () => {
	assert.equal(FileTargets.isDeclarationFile('types.d.ts'), true);

	assert.equal(FileTargets.isDeclarationFile('app.ts'), false);
});
