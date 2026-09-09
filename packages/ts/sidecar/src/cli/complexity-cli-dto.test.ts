import assert from 'node:assert/strict';
import { test } from 'node:test';
import { ComplexityCliDto } from '#sidecar/cli/complexity-cli-dto';

test('the scan accepts the TypeScript and JavaScript families', () => {
	for (const path of ['src/app.ts', 'src/Screen.tsx', 'src/app.mts', 'src/app.cts', 'src/app.js', 'src/app.jsx']) {
		assert.equal(ComplexityCliDto.isScorable(path), true, path);
	}
});

test('the scan skips declarations, tests, and everything it cannot parse', () => {
	for (const path of ['src/types.d.ts', 'src/app.test.ts', 'src/app.spec.tsx', 'src/app.vue', 'docs/notes.md', 'main.go']) {
		assert.equal(ComplexityCliDto.isScorable(path), false, path);
	}
});

test('the command line carries the root, the listing, and the direct files', () => {
	const options = ComplexityCliDto.parse(['--root', '/repo', '--files-from', '/tmp/list', 'src/app.ts']);

	assert.equal(options.root, '/repo');
	assert.equal(options.filesFrom, '/tmp/list');
	assert.deepEqual([...options.files], ['src/app.ts']);
});

test('an omitted root falls back to the working directory', () => {
	assert.equal(ComplexityCliDto.parse([]).root, process.cwd());
	assert.equal(ComplexityCliDto.parse([]).filesFrom, '');
});
