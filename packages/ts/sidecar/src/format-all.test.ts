import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { chmod, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { availableParallelism, tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { test } from 'node:test';
import { promisify } from 'node:util';
import { CliOptionsDto } from '#sidecar/format-all';
import { FormatPipeline } from '#sidecar/pipeline/format-pipeline';
import { mapPool } from '#sidecar/kernel/concurrency';
import { NodeProcessRunner } from '#sidecar/io/process-runner';
import { isErr } from '#sidecar/kernel/result';
import { PipelineFactory } from '#sidecar/pipeline/pipeline-factory';
import { SourceFileEditor } from '#sidecar/pipeline/source-file-editor';
import { NodeSourceFiles } from '#sidecar/io/source-files';

const execFileAsync = promisify(execFile);
const formatAllScript = resolve(import.meta.dirname, 'format-all.ts');
const factory = PipelineFactory.create();
const sourceFiles = new NodeSourceFiles();

const pipeline = new FormatPipeline({
	editor: new SourceFileEditor({ sourceFiles }),
	processRunner: new NodeProcessRunner(),
	validator: factory.syntaxValidator(sourceFiles),
});

const segmentFormatter = factory.segmentFormatter();

test('parseArgs splits flags and file sections', () => {
	const options = CliOptionsDto.parse(['--check', '--oxfmt-bin', '/bin/oxfmt', '--oxfmt-config', '/etc/oxfmtrc.json', '--format-files', 'a.ts', 'b.vue', '--syntax-files', 'a.ts', 'types.d.ts']);

	assert.equal(isErr(options), false);

	assert.deepEqual(!isErr(options) && options.value, {
		mode: 'check',
		oxfmtBin: '/bin/oxfmt',
		oxfmtConfig: '/etc/oxfmtrc.json',
		formatFiles: ['a.ts', 'b.vue'],
		syntaxFiles: ['a.ts', 'types.d.ts'],
	});
});

test('parseArgs accepts empty file sections and rejects stray arguments', () => {
	const options = CliOptionsDto.parse(['--format-files', '--syntax-files']);

	assert.equal(isErr(options), false);

	if (isErr(options)) {
		return;
	}

	assert.deepEqual(options.value.formatFiles, []);

	assert.deepEqual(options.value.syntaxFiles, []);

	const unexpected = CliOptionsDto.parse(['stray.ts']);

	assert.ok(isErr(unexpected));

	assert.match(isErr(unexpected) ? unexpected.error.message : '', /unexpected argument/);
});

test('mapPool processes every item, preserves order, and honors the limit', async () => {
	let active = 0;

	let peak = 0;

	const items = Array.from({ length: 20 }, (_, index) => {
		return index;
	});

	const outcomes = await mapPool(
		items,
		availableParallelism(),
		async (item) => {
			active++;
			peak = Math.max(peak, active);

			await new Promise((resolvePromise) => {
				return setTimeout(resolvePromise, 1);
			});

			active--;

			return item % 2 === 0;
		},
	);

	assert.deepEqual(
		outcomes,
		items.map((item) => {
			return item % 2 === 0;
		}),
	);

	assert.ok(peak <= items.length, `peak concurrency ${peak} exceeded item count`);
});

test('runPass processes every file and skips missing ones', async () => {
	const dir = await mkdtemp(
		join(
			tmpdir(),
			'fmtkit-sidecar-runpass-',
		),
	);

	try {
		const first = join(dir, 'a.ts');
		const missing = join(dir, 'missing.ts');
		const last = join(dir, 'b.ts');

		await writeFile(first, 'const value = 1;\n');

		await writeFile(last, 'const value = 1;\n');

		const outcomes = await pipeline.runPass(segmentFormatter, [first, missing, last], 'write');

		assert.equal(outcomes[0]?.error, null);

		assert.equal(outcomes[2]?.error, null);

		assert.equal(outcomes[1]?.error?._tag === 'SourceFileUnreadable' && outcomes[1].error.isNotFound(), true);
	} finally {
		await rm(
			dir,
			{ recursive: true, force: true },
		);
	}
});

test('runOxfmt resolves without spawning when no binary or no files are given', async () => {
	assert.equal(isErr(await pipeline.runOxfmt({ mode: 'write', bin: null, config: null, files: ['a.ts'] })), false);

	assert.equal(isErr(await pipeline.runOxfmt({ mode: 'write', bin: 'false', config: null, files: [] })), false);
});

test('runOxfmt spawns with --check in check mode and surfaces failures', async () => {
	assert.equal(isErr(await pipeline.runOxfmt({ mode: 'check', bin: 'true', config: null, files: ['a.ts'] })), false);

	const failed = await pipeline.runOxfmt({ mode: 'check', bin: 'false', config: null, files: ['a.ts'] });

	assert.ok(isErr(failed));

	assert.match(isErr(failed) ? failed.error.message : '', /oxfmt exited/);
});

test('runOxfmt surfaces the exit status of the spawned formatter', async () => {
	assert.equal(isErr(await pipeline.runOxfmt({ mode: 'write', bin: 'true', config: null, files: ['a.ts'] })), false);

	const failed = await pipeline.runOxfmt({ mode: 'write', bin: 'false', config: null, files: ['a.ts'] });

	assert.ok(isErr(failed));

	assert.equal(isErr(failed) ? failed.error.code : 0, 1);
});

test('runOxfmt chunks large file lists', async () => {
	const dir = await mkdtemp(
		join(
			tmpdir(),
			'fmtkit-sidecar-oxfmt-chunks-',
		),
	);

	try {
		const bin = join(dir, 'oxfmt');
		const log = join(dir, 'args.log');

		const files = Array.from({ length: 205 }, (_, index) => {
			return `file-${index}.ts`;
		});

		await writeFile(
			bin,
			`#!/usr/bin/env bash
printf '%s\\n' "$#" >> "${log}"
`,
		);

		await chmod(bin, 0o755);

		assert.equal(isErr(await pipeline.runOxfmt({ mode: 'write', bin, config: null, files })), false);

		assert.deepEqual((await readFile(log, 'utf8')).trim().split('\n'), ['102', '102', '7']);
	} finally {
		await rm(
			dir,
			{ recursive: true, force: true },
		);
	}
});

test('format-all pipeline formats files and exits 0 end-to-end', async () => {
	const dir = await mkdtemp(
		join(
			tmpdir(),
			'fmtkit-sidecar-format-all-',
		),
	);

	try {
		const file = join(dir, 'app.ts');

		await writeFile(file, 'function run() {\n\tconst x = 1;\n\tif (x) return x;\n\treturn 0;\n}\n');

		const { stdout } = await execFileAsync(
			process.execPath,
			['--import', 'tsx', formatAllScript, '--format-files', file, '--syntax-files', file],
		);

		const updated = await readFile(file, 'utf8');

		assert.match(updated, /if \(x\) \{\n\t\treturn x;\n\t\}/);

		const blankIndex = stdout.indexOf('[blank-lines]');
		const fluentIndex = stdout.indexOf('[fluent-chains]');
		const validateIndex = stdout.indexOf('[validate-syntax]');

		assert.ok(blankIndex >= 0 && fluentIndex > blankIndex && validateIndex > fluentIndex, `unexpected pass order:\n${stdout}`);
	} finally {
		await rm(
			dir,
			{ recursive: true, force: true },
		);
	}
});

test('format-all pipeline reaches a fixed point in a single run', async () => {
	const dir = await mkdtemp(
		join(
			tmpdir(),
			'fmtkit-sidecar-format-all-fixpoint-',
		),
	);

	try {
		const file = join(dir, 'app.ts');

		// A single-line statement followed by a call that expanded-calls turns
		// multiline: the blank line separating them can only be inserted after
		// the expansion has happened.
		await writeFile(
			file,
			"function queueMessage(id: string, body: object) {\n\treturn { id, body };\n}\n\nexport function run() {\n\tconst registry = new Map<string, string>();\n\tconst invalid = queueMessage('1', { unexpected: true });\n\treturn [registry, invalid];\n}\n",
		);

		await execFileAsync(
			process.execPath,
			['--import', 'tsx', formatAllScript, '--format-files', file, '--syntax-files', file],
		);

		const afterFirstRun = await readFile(file, 'utf8');

		assert.match(afterFirstRun, /queueMessage\(\n\t\t'1',/, 'expected the call to be expanded');

		await execFileAsync(
			process.execPath,
			['--import', 'tsx', formatAllScript, '--format-files', file, '--syntax-files', file],
		);

		assert.equal(await readFile(file, 'utf8'), afterFirstRun, 'second run must not change an already formatted file');
	} finally {
		await rm(
			dir,
			{ recursive: true, force: true },
		);
	}
});

test('format-all pipeline deduplicates repeated input paths', async () => {
	const dir = await mkdtemp(
		join(
			tmpdir(),
			'fmtkit-sidecar-format-all-dup-',
		),
	);

	try {
		const file = join(dir, 'app.ts');

		await writeFile(file, 'const one = 1;\n');

		const { stdout } = await execFileAsync(
			process.execPath,
			['--import', 'tsx', formatAllScript, '--format-files', file, file, '--syntax-files', file, file],
		);

		assert.match(stdout, /\[blank-lines\] processed 1 file\(s\)/);

		assert.match(stdout, /\[validate-syntax\] checked 1 file\(s\)/);
	} finally {
		await rm(
			dir,
			{ recursive: true, force: true },
		);
	}
});

test('format-all pipeline formats embedded blocks in html and markdown hosts', async () => {
	const dir = await mkdtemp(
		join(
			tmpdir(),
			'fmtkit-sidecar-format-all-hosts-',
		),
	);

	try {
		const html = join(dir, 'page.html');
		const markdown = join(dir, 'notes.md');
		const chain = "export const meRoutes = createRouter().use('*', bindEnv).use(identityMiddleware).get('/', getMe);";

		await writeFile(html, `<body>\n<script>\n${chain}\n</script>\n</body>\n`);

		await writeFile(markdown, `# Doc\n\n\`\`\`ts\n${chain}\n\`\`\`\n`);

		await execFileAsync(
			process.execPath,
			['--import', 'tsx', formatAllScript, '--format-files', html, markdown, '--syntax-files', html, markdown],
		);

		const splitChain = /createRouter\(\)\n\t\.use\('\*', bindEnv\)\n\t\.use\(identityMiddleware\)\n\t\.get\('\/', getMe\);/;

		assert.match(await readFile(html, 'utf8'), splitChain);

		assert.match(await readFile(markdown, 'utf8'), splitChain);
	} finally {
		await rm(
			dir,
			{ recursive: true, force: true },
		);
	}
});

test('format-all pipeline tolerates syntactically broken markdown fences', async () => {
	const dir = await mkdtemp(
		join(
			tmpdir(),
			'fmtkit-sidecar-format-all-md-broken-',
		),
	);

	try {
		const markdown = join(dir, 'broken.md');

		await writeFile(markdown, '# Doc\n\n```ts\nconst broken = {;\n```\n');

		const { stdout } = await execFileAsync(
			process.execPath,
			['--import', 'tsx', formatAllScript, '--format-files', markdown, '--syntax-files', markdown],
		);

		assert.match(stdout, /\[validate-syntax\] checked 1 file\(s\)/);
	} finally {
		await rm(
			dir,
			{ recursive: true, force: true },
		);
	}
});

test('format-all pipeline exits 1 for syntactically broken html script blocks', async () => {
	const dir = await mkdtemp(
		join(
			tmpdir(),
			'fmtkit-sidecar-format-all-html-broken-',
		),
	);

	try {
		const html = join(dir, 'broken.html');

		await writeFile(html, '<body>\n<script>\nconst broken = {;\n</script>\n</body>\n');

		await assert.rejects(execFileAsync(process.execPath, ['--import', 'tsx', formatAllScript, '--format-files', html, '--syntax-files', html]), (err: { code?: number }) => {
			return err.code === 1;
		});
	} finally {
		await rm(
			dir,
			{ recursive: true, force: true },
		);
	}
});

test('format-all pipeline exits 1 when validation finds syntax errors', async () => {
	const dir = await mkdtemp(
		join(
			tmpdir(),
			'fmtkit-sidecar-format-all-bad-',
		),
	);

	try {
		const file = join(dir, 'broken.ts');

		await writeFile(file, 'const broken = {;\n');

		await assert.rejects(execFileAsync(process.execPath, ['--import', 'tsx', formatAllScript, '--format-files', file, '--syntax-files', file]), (err: { code?: number }) => {
			return err.code === 1;
		});
	} finally {
		await rm(
			dir,
			{ recursive: true, force: true },
		);
	}
});
