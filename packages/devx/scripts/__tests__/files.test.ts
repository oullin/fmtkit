import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { dirExists, listSourceFiles, processFile } from '#devx/files';

async function withTempDir(fn: (dir: string) => Promise<void>): Promise<void> {
	const dir = await mkdtemp(join(tmpdir(), 'go-fmt-devx-files-'));

	try {
		await fn(dir);
	} finally {
		await rm(dir, { recursive: true, force: true });
	}
}

test('dirExists reports existing directories and missing paths', async () => {
	await withTempDir(async (dir) => {
		assert.equal(await dirExists(dir), true);

		assert.equal(await dirExists(join(dir, 'missing')), false);
	});
});

test('listSourceFiles returns TypeScript and Vue files only', async () => {
	await withTempDir(async (dir) => {
		await writeFile(join(dir, 'component.vue'), '<script setup lang="ts">\nconst value = 1;\n</script>\n');

		await writeFile(join(dir, 'source.ts'), 'const value = 1;\n');

		await writeFile(join(dir, 'notes.md'), '# Notes\n');

		const files = (await listSourceFiles(dir)).map((file) => {
			return file.slice(dir.length + 1);
		});

		assert.deepEqual(files.sort(), ['component.vue', 'source.ts']);
	});
});

test('processFile reports check changes without writing TypeScript files', async () => {
	await withTempDir(async (dir) => {
		const file = join(dir, 'source.ts');
		const original = ['function run() {', '\tconst value = 1;', '\tif (value) return value;', '}', ''].join('\n');

		await writeFile(file, original);

		assert.equal(await processFile(file, true), true);

		assert.equal(await readFile(file, 'utf8'), original);
	});
});

test('processFile rewrites Vue script blocks and reports unchanged files', async () => {
	await withTempDir(async (dir) => {
		const file = join(dir, 'component.vue');

		await writeFile(file, ['<script setup lang="ts">', 'const value = 1;', 'if (value) console.log(value);', '</script>', ''].join('\n'));

		assert.equal(await processFile(file, false), true);

		const updated = await readFile(file, 'utf8');

		assert.match(updated, /if \(value\) \{\n\tconsole\.log\(value\);\n\}/);

		assert.equal(await processFile(file, true), false);
	});
});
