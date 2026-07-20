import assert from 'node:assert/strict';
import { mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { SourceFileUnreadable } from '#sidecar/errors';
import { isErr } from '#sidecar/result';
import { NodeSourceFiles } from '#sidecar/source-files';

test('SourceFileUnreadable identifies ENOENT-shaped causes only', () => {
	assert.equal(new SourceFileUnreadable('missing.ts', { code: 'ENOENT' }).isNotFound(), true);

	assert.equal(new SourceFileUnreadable('denied.ts', { code: 'EACCES' }).isNotFound(), false);

	assert.equal(new SourceFileUnreadable('failed.ts', new Error('nope')).isNotFound(), false);
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

test('NodeSourceFiles carries read and write failures', async () => {
	const dir = await mkdtemp(
		join(
			tmpdir(),
			'fmtkit-sidecar-failures-',
		),
	);

	try {
		const missing = join(dir, 'missing', 'target.ts');
		const sourceFiles = new NodeSourceFiles();

		const read = await sourceFiles.readText(missing);

		const written = await sourceFiles.writeTextAtomic(missing, 'content\n');

		assert.equal(isErr(read) && read.error._tag, 'SourceFileUnreadable');

		assert.equal(isErr(written) && written.error._tag, 'SourceFileUnwritable');

		assert.equal(isErr(written) && written.error.path, missing);
	} finally {
		await rm(
			dir,
			{ recursive: true, force: true },
		);
	}
});
