import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm, unlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const script = fileURLToPath(import.meta.resolve('#devx/blank-lines'));
const tsx = fileURLToPath(import.meta.resolve('tsx/cli'));

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
			run(process.execPath, [tsx, script, 'AgentDock.vue'], dir);

			const output = await readFile(join(dir, 'AgentDock.vue'), 'utf8');

			assert.match(output, /const hoveredItem = computed\(\(\) => \{\n    const id = hoveredId\.value;\n\n    if \(!id\) \{/);
			assert.match(output, /return null;\n    \}\n\n    if \(id in systemLabels\) \{/);
			assert.match(output, /shortcut: undefined \};\n    \}\n\n    return props\.items/);
		},
	);
});

test('keeps adjacent computed declarations syntactically valid', async () => {
	await withFixture(
		{
			'useAppController.ts': [
				'import { computed, ref } from "vue";',
				'',
				'export function useAppController() {',
				'\tconst searchQuery = ref("");',
				'\tconst debouncedSearch = ref("");',
				'\tconst normalizedSearch = computed(() => searchQuery.value.trim().toLowerCase());',
				'\tconst normalizedDebouncedSearch = computed(() => debouncedSearch.value.trim().toLowerCase());',
				'',
				'\treturn { normalizedSearch, normalizedDebouncedSearch };',
				'}',
				'',
			].join('\n'),
		},
		async (dir) => {
			run(process.execPath, [tsx, script, 'useAppController.ts'], dir);
			run(process.execPath, [tsx, script, 'useAppController.ts'], dir);

			const output = await readFile(join(dir, 'useAppController.ts'), 'utf8');

			assert.doesNotMatch(output, /\);\);/);
			assert.match(output, /const normalizedDebouncedSearch = computed\(\(\) => debouncedSearch\.value\.trim\(\)\.toLowerCase\(\)\);/);
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
			run(process.execPath, [tsx, script, 'tracked.ts', 'types.d.ts'], dir);

			const tracked = await readFile(join(dir, 'tracked.ts'), 'utf8');

			const ignored = await readFile(join(dir, 'ignored.ts'), 'utf8');

			const types = await readFile(join(dir, 'types.d.ts'), 'utf8');

			assert.match(tracked, /const value = 1;\n\n\tif \(value\) \{\n\t\treturn value;\n\t\}\n\n\treturn 0;/);
			assert.doesNotMatch(ignored, /const value = 1;\n\n\tif/);
			assert.doesNotMatch(types, /value: string;\n\ndeclare function/);
		},
	);
});

test('skips tracked files missing from the working tree', async () => {
	await withFixture(
		{
			'deleted.ts': ['function run() {', '\tconst value = 1;', '\treturn value;', '}', ''].join('\n'),
			'kept.ts': ['function run() {', '\tconst value = 1;', '\treturn value;', '}', ''].join('\n'),
		},
		async (dir) => {
			await unlink(join(dir, 'deleted.ts'));

			run(process.execPath, [tsx, script, 'deleted.ts', 'kept.ts'], dir);

			const kept = await readFile(join(dir, 'kept.ts'), 'utf8');

			assert.match(kept, /const value = 1;\n\n\treturn value;/);
		},
	);
});

test('CLI uses modular formatter for body wrapping and declaration ordering', async () => {
	await withFixture(
		{
			'tracked.ts': ['import {', '\ta,', '} from "a";', 'import { b } from "b";', 'function run() {', '\tif (ready) done();', '}', ''].join('\n'),
		},
		async (dir) => {
			run(process.execPath, [tsx, script, 'tracked.ts'], dir);

			const tracked = await readFile(join(dir, 'tracked.ts'), 'utf8');

			assert.match(tracked, /import \{ b \} from "b";\n\nimport \{\n\ta,\n\} from "a";/);
			assert.match(tracked, /if \(ready\) \{\n\t\tdone\(\);\n\t\}/);
		},
	);
});
