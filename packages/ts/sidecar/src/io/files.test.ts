import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { Files } from '#sidecar/io/files';
import { FormatPipeline } from '#sidecar/format-pipeline';
import { NodeProcessRunner } from '#sidecar/io/process-runner';
import { isErr } from '#sidecar/kernel/result';
import { NodeSourceFiles } from '#sidecar/io/source-files';

const pipeline = new FormatPipeline({ sourceFiles: new NodeSourceFiles(), processRunner: new NodeProcessRunner() });

async function processFile(file: string, mode: 'check' | 'write'): Promise<boolean> {
	const outcome = await pipeline.formatFile(file, mode);

	assert.equal(isErr(outcome), false);

	return isErr(outcome) ? false : outcome.value;
}

async function withTempDir(fn: (dir: string) => Promise<void>): Promise<void> {
	const dir = await mkdtemp(
		join(
			tmpdir(),
			'fmtkit-sidecar-files-',
		),
	);

	try {
		await fn(dir);
	} finally {
		await rm(
			dir,
			{ recursive: true, force: true },
		);
	}
}

test('dirExists reports existing directories and missing paths', async () => {
	await withTempDir(async (dir) => {
		assert.equal(await Files.dirExists(dir), true);

		assert.equal(await Files.dirExists(join(dir, 'missing')), false);
	});
});

test('listSourceFiles returns TypeScript and Vue files only', async () => {
	await withTempDir(async (dir) => {
		await writeFile(
			join(dir, 'component.vue'),
			'<script setup lang="ts">\nconst value = 1;\n</script>\n',
		);

		await writeFile(
			join(dir, 'source.ts'),
			'const value = 1;\n',
		);

		await writeFile(
			join(dir, 'notes.md'),
			'# Notes\n',
		);

		const files = (await Files.listSourceFiles(dir)).map((file) => {
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

		assert.equal(await processFile(file, 'check'), true);

		assert.equal(await readFile(file, 'utf8'), original);
	});
});

test('processFile leaves non-JS/TS Vue script blocks untouched', async () => {
	await withTempDir(async (dir) => {
		const file = join(dir, 'component.vue');
		const yamlBlock = ['<script lang="yaml">', 'items:', '  - if (value) console.log(value)', '</script>'].join('\n');
		const tsBlock = ['<script setup lang="ts">', 'const value = 1;', 'if (value) console.log(value);', '</script>'].join('\n');

		await writeFile(file, `${yamlBlock}\n${tsBlock}\n`);

		assert.equal(await processFile(file, 'write'), true);

		const updated = await readFile(file, 'utf8');

		assert.ok(updated.startsWith(yamlBlock), 'yaml script block must not be reformatted');

		assert.match(updated, /if \(value\) \{\n\tconsole\.log\(value\);\n\}/);
	});
});

test('processFile rewrites Vue script blocks and reports unchanged files', async () => {
	await withTempDir(async (dir) => {
		const file = join(dir, 'component.vue');

		await writeFile(
			file,
			['<script setup lang="ts">', 'const value = 1;', 'if (value) console.log(value);', '</script>', ''].join('\n'),
		);

		assert.equal(await processFile(file, 'write'), true);

		const updated = await readFile(file, 'utf8');

		assert.match(updated, /if \(value\) \{\n\tconsole\.log\(value\);\n\}/);

		assert.equal(await processFile(file, 'check'), false);
	});
});
