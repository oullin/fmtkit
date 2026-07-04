import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { test } from 'node:test';
import { promisify } from 'node:util';
import { mapPool, parseArgs, runOxfmt, runPass } from '#devx/format-all';

const execFileAsync = promisify(execFile);
const tsxBin = resolve(import.meta.dirname, '../node_modules/.bin/tsx');
const formatAllScript = resolve(import.meta.dirname, 'format-all.ts');

test('parseArgs splits flags and file sections', () => {
	const options = parseArgs(['--check', '--oxfmt-bin', '/bin/oxfmt', '--oxfmt-config', '/etc/oxfmtrc.json', '--format-files', 'a.ts', 'b.vue', '--syntax-files', 'a.ts', 'types.d.ts']);

	assert.deepEqual(options, {
		check: true,
		oxfmtBin: '/bin/oxfmt',
		oxfmtConfig: '/etc/oxfmtrc.json',
		formatFiles: ['a.ts', 'b.vue'],
		syntaxFiles: ['a.ts', 'types.d.ts'],
	});
});

test('parseArgs accepts empty file sections and rejects stray arguments', () => {
	const options = parseArgs(['--format-files', '--syntax-files']);

	assert.deepEqual(options.formatFiles, []);

	assert.deepEqual(options.syntaxFiles, []);

	assert.throws(() => {
		return parseArgs(['stray.ts']);
	}, /unexpected argument/);
});

test('mapPool processes every item, preserves order, and honors the limit', async () => {
	let active = 0;

	let peak = 0;

	const items = Array.from({ length: 20 }, (_, index) => {
		return index;
	});

	const results = await mapPool(items, 3, async (item) => {
		active++;
		peak = Math.max(peak, active);

		await new Promise((resolvePromise) => {
			return setTimeout(resolvePromise, 1);
		});

		active--;

		return item * 2;
	});

	assert.deepEqual(
		results,
		items.map((item) => {
			return item * 2;
		}),
	);

	assert.ok(peak <= 3, `peak concurrency ${peak} exceeded limit`);
});

test('runPass processes every file and skips missing ones', async () => {
	const seen: string[] = [];

	await runPass('blank-lines', ['a.ts', 'missing.ts', 'b.ts'], false, async (file) => {
		if (file === 'missing.ts') {
			throw Object.assign(new Error('gone'), { code: 'ENOENT' });
		}

		seen.push(file);

		return true;
	});

	assert.deepEqual(seen.sort(), ['a.ts', 'b.ts']);
});

test('runOxfmt resolves without spawning in check mode or without a binary', async () => {
	await runOxfmt({ check: true, oxfmtBin: '/bin/false', oxfmtConfig: null, formatFiles: ['a.ts'], syntaxFiles: [] });

	await runOxfmt({ check: false, oxfmtBin: null, oxfmtConfig: null, formatFiles: ['a.ts'], syntaxFiles: [] });
});

test('runOxfmt surfaces the exit status of the spawned formatter', async () => {
	await runOxfmt({ check: false, oxfmtBin: 'true', oxfmtConfig: null, formatFiles: ['a.ts'], syntaxFiles: [] });

	await assert.rejects(runOxfmt({ check: false, oxfmtBin: 'false', oxfmtConfig: null, formatFiles: ['a.ts'], syntaxFiles: [] }), /oxfmt exited/);
});

test('format-all pipeline formats files and exits 0 end-to-end', async () => {
	const dir = await mkdtemp(join(tmpdir(), 'go-fmt-devx-format-all-'));

	try {
		const file = join(dir, 'app.ts');

		await writeFile(file, 'function run() {\n\tconst x = 1;\n\tif (x) return x;\n\treturn 0;\n}\n');

		const { stdout } = await execFileAsync(tsxBin, [formatAllScript, '--format-files', file, '--syntax-files', file]);

		const updated = await readFile(file, 'utf8');

		assert.match(updated, /if \(x\) \{\n\t\treturn x;\n\t\}/);

		const blankIndex = stdout.indexOf('[blank-lines]');
		const fluentIndex = stdout.indexOf('[fluent-chains]');
		const validateIndex = stdout.indexOf('[validate-syntax]');

		assert.ok(blankIndex >= 0 && fluentIndex > blankIndex && validateIndex > fluentIndex, `unexpected pass order:\n${stdout}`);
	} finally {
		await rm(dir, { recursive: true, force: true });
	}
});

test('format-all pipeline exits 1 when validation finds syntax errors', async () => {
	const dir = await mkdtemp(join(tmpdir(), 'go-fmt-devx-format-all-bad-'));

	try {
		const file = join(dir, 'broken.ts');

		await writeFile(file, 'const broken = {;\n');

		await assert.rejects(execFileAsync(tsxBin, [formatAllScript, '--format-files', file, '--syntax-files', file]), (err: { code?: number }) => {
			return err.code === 1;
		});
	} finally {
		await rm(dir, { recursive: true, force: true });
	}
});
