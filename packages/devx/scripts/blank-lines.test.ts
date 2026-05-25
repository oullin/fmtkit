import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const repoRoot = fileURLToPath(new URL('../../..', import.meta.url));
const script = join(repoRoot, 'packages/devx/scripts/blank-lines.ts');
const tsx = join(repoRoot, 'packages/devx/node_modules/.bin/tsx');

function run(command: string, args: string[], cwd: string): void {
	const result = spawnSync(command, args, { cwd, encoding: 'utf8' });

	assert.equal(result.status, 0, result.stderr || result.stdout);
}

async function withFixture(files: Record<string, string>, fn: (dir: string) => Promise<void>): Promise<void> {
	const dir = await mkdtemp(join(tmpdir(), 'go-fmt-blank-lines-'));

	try {
		run('git', ['init', '-q'], dir);

		for (const [file, content] of Object.entries(files)) {
			await writeFile(join(dir, file), content);
		}

		run('git', ['add', '.'], dir);
		await fn(dir);
	} finally {
		await rm(dir, { recursive: true, force: true });
	}
}

test('adds expected blank lines in Vue script blocks', async () => {
	await withFixture(
		{
			'AgentDock.vue': [
				'<script setup lang="ts">',
				'const hoveredItem = computed(() => {',
				'    const id = hoveredId.value;',
				'    if (!id) return null;',
				'    if (id in systemLabels) return { id, label: systemLabels[id], shortcut: undefined };',
				'    return props.items.find((i) => i.id === id) ?? null;',
				'});',
				'</script>',
				'',
			].join('\n'),
		},
		async (dir) => {
			run(tsx, [script, '.'], dir);

			const output = await readFile(join(dir, 'AgentDock.vue'), 'utf8');

			assert.match(output, /const id = hoveredId\.value;\n\n    if \(!id\) return null;\n\n    if \(id in systemLabels\)/);
			assert.match(output, /shortcut: undefined \};\n\n    return props\.items/);
		},
	);
});

test('ignores untracked ignored files and declaration files', async () => {
	await withFixture(
		{
			'.gitignore': 'ignored.ts\n',
			'tracked.ts': ['function run() {', '\tconst value = 1;', '\tif (value) return value;', '\treturn 0;', '}', ''].join('\n'),
			'ignored.ts': ['function run() {', '\tconst value = 1;', '\tif (value) return value;', '}', ''].join('\n'),
			'types.d.ts': ['declare const value: string;', 'declare function run(): string;', ''].join('\n'),
		},
		async (dir) => {
			run(tsx, [script, '.'], dir);

			const tracked = await readFile(join(dir, 'tracked.ts'), 'utf8');
			const ignored = await readFile(join(dir, 'ignored.ts'), 'utf8');
			const types = await readFile(join(dir, 'types.d.ts'), 'utf8');

			assert.match(tracked, /const value = 1;\n\n\tif \(value\) return value;\n\n\treturn 0;/);
			assert.doesNotMatch(ignored, /const value = 1;\n\n\tif/);
			assert.doesNotMatch(types, /value: string;\n\ndeclare function/);
		},
	);
});
