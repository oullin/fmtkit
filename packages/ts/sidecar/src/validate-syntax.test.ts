import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const script = fileURLToPath(
	import.meta.resolve('#sidecar/validate-syntax'),
);
const tsx = fileURLToPath(
	import.meta.resolve('tsx'),
);

async function withFixture(files: Record<string, string>, fn: (dir: string) => Promise<void>): Promise<void> {
	const dir = await mkdtemp(
		join(
			tmpdir(),
			'fmtkit-validate-syntax-',
		),
	);

	try {
		assert.equal(spawnSync('git', ['init', '-q'], { cwd: dir }).status, 0);

		for (const [file, content] of Object.entries(files)) {
			await writeFile(
				join(dir, file),
				content,
			);
		}

		assert.equal(spawnSync('git', ['add', '.'], { cwd: dir }).status, 0);

		await fn(dir);
	} finally {
		await rm(
			dir,
			{ recursive: true, force: true },
		);
	}
}

test('accepts valid TypeScript and Vue script blocks', async () => {
	await withFixture(
		{
				'valid.ts': 'const value = computed(() => source.value.trim().toLowerCase());\n',
				'Valid.vue': '<script setup lang="ts">\nconst value = 1;\n</script>\n<template>{{ value }}</template>\n',
			},
		async (dir) => {
				const result = spawnSync(process.execPath, ['--import', tsx, script, 'valid.ts', 'Valid.vue'], { cwd: dir, encoding: 'utf8' });

				assert.equal(result.status, 0, result.stderr || result.stdout);
				assert.match(result.stdout, /\[validate-syntax\] checked 2 file\(s\)/);
			},
	);
});

test('accepts Vue TSX and JSX script blocks', async () => {
	await withFixture(
		{
				'Component.vue': '<script setup lang="tsx">\nconst view = <section>Ready</section>;\n</script>\n',
				'Legacy.vue': "<script lang='jsx'>\nconst view = <section>Ready</section>;\n</script>\n",
			},
		async (dir) => {
				const result = spawnSync(process.execPath, ['--import', tsx, script, 'Component.vue', 'Legacy.vue'], { cwd: dir, encoding: 'utf8' });

				assert.equal(result.status, 0, result.stderr || result.stdout);
				assert.match(result.stdout, /\[validate-syntax\] checked 2 file\(s\)/);
			},
	);
});

test('reports Vue syntax errors on original file lines', async () => {
	await withFixture(
		{
				'Broken.vue': '<template>\n\t<div />\n</template>\n<script setup lang="ts">\nconst broken = ;\n</script>\n',
			},
		async (dir) => {
				const result = spawnSync(process.execPath, ['--import', tsx, script, 'Broken.vue'], { cwd: dir, encoding: 'utf8' });

				assert.equal(result.status, 1);
				assert.match(result.stderr, /\[validate-syntax\].*Broken\.vue/);
				assert.match(result.stderr, /5 \| const broken = ;/);
			},
	);
});

test('fails with a clear diagnostic for corrupted formatter output', async () => {
	await withFixture(
		{
				'useAppController.ts': 'const normalizedDebouncedSearch = computed(() => debouncedSearch.value.trim().toLowerCase()););\n',
			},
		async (dir) => {
				const result = spawnSync(process.execPath, ['--import', tsx, script, 'useAppController.ts'], { cwd: dir, encoding: 'utf8' });

				assert.equal(result.status, 1);
				assert.match(result.stderr, /\[validate-syntax\].*useAppController\.ts/);
				assert.match(result.stderr, /Unexpected token/);
			},
	);
});

test('skips missing paths and ignores non-source arguments', async () => {
	await withFixture(
		{
				'notes.md': '# Notes\n',
			},
		async (dir) => {
				const result = spawnSync(process.execPath, ['--import', tsx, script, 'missing.ts', 'notes.md'], { cwd: dir, encoding: 'utf8' });

				assert.equal(result.status, 0, result.stderr || result.stdout);
				assert.match(result.stderr, /\[validate-syntax\] path not found, skipping: missing\.ts/);
				assert.match(result.stdout, /\[validate-syntax\] checked 1 file\(s\)/);
			},
	);
});
