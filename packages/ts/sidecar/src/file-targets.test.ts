import assert from 'node:assert/strict';
import { test } from 'node:test';
import { FileTargets } from '#sidecar/file-targets';

test('isTargetFile accepts ts and vue but not declarations', () => {
	assert.equal(FileTargets.isTargetFile('app.ts'), true);

	assert.equal(FileTargets.isTargetFile('widget.vue'), true);

	assert.equal(FileTargets.isTargetFile('types.d.ts'), false);

	assert.equal(FileTargets.isTargetFile('notes.md'), false);
});

test('isDeclarationFile only matches .d.ts', () => {
	assert.equal(FileTargets.isDeclarationFile('types.d.ts'), true);

	assert.equal(FileTargets.isDeclarationFile('app.ts'), false);
});
