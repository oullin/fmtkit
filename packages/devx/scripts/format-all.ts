import { spawn } from 'node:child_process';
import { availableParallelism } from 'node:os';
import { pathToFileURL } from 'node:url';
import { processFile as processBlankLinesFile } from '#devx/files';
import { processFluentChainsFile } from '#devx/fluent-chains';
import { isNotFoundError, isTargetFile } from '#devx/pass-utils';
import { validateFile } from '#devx/validate-syntax';

// format-all runs the full TS pipeline (blank-lines → oxfmt → fluent-chains
// → oxfmt → validate-syntax) inside a single process. Files within a pass are
// processed concurrently; results are reported in input order so the output
// stays deterministic.

type CliOptions = {
	check: boolean;
	oxfmtBin: string | null;
	oxfmtConfig: string | null;
	formatFiles: string[];
	syntaxFiles: string[];
};

type PassOutcome = {
	file: string;
	changed: boolean;
	missing: boolean;
};

const OXFMT_CHUNK_SIZE = 100;

export function parseArgs(argv: string[]): CliOptions {
	const options: CliOptions = {
		check: false,
		oxfmtBin: null,
		oxfmtConfig: null,
		formatFiles: [],
		syntaxFiles: [],
	};

	let section: 'formatFiles' | 'syntaxFiles' | null = null;

	for (let i = 0; i < argv.length; i++) {
		const arg = argv[i];

		if (arg === '--check') {
			options.check = true;
			section = null;
		} else if (arg === '--oxfmt-bin') {
			options.oxfmtBin = argv[++i] ?? null;
			section = null;
		} else if (arg === '--oxfmt-config') {
			options.oxfmtConfig = argv[++i] ?? null;
			section = null;
		} else if (arg === '--format-files') {
			section = 'formatFiles';
		} else if (arg === '--syntax-files') {
			section = 'syntaxFiles';
		} else if (section) {
			options[section].push(arg);
		} else {
			throw new Error(`[format-all] unexpected argument: ${arg}`);
		}
	}

	return options;
}

export async function mapPool<T, R>(items: T[], limit: number, fn: (item: T) => Promise<R>): Promise<R[]> {
	const results = new Array<R>(items.length);

	let nextIndex = 0;

	async function worker(): Promise<void> {
		while (true) {
			const index = nextIndex++;

			if (index >= items.length) {
				return;
			}

			results[index] = await fn(items[index]);
		}
	}

	const workers = Array.from({ length: Math.max(1, Math.min(limit, items.length)) }, worker);

	await Promise.all(workers);

	return results;
}

export async function runPass(label: string, files: string[], check: boolean, processOne: (file: string, check: boolean) => Promise<boolean>): Promise<void> {
	const outcomes = await mapPool(files, availableParallelism(), async (file): Promise<PassOutcome> => {
		try {
			return { file, changed: await processOne(file, check), missing: false };
		} catch (err) {
			if (isNotFoundError(err)) {
				return { file, changed: false, missing: true };
			}

			throw err;
		}
	});

	let changedCount = 0;

	for (const outcome of outcomes) {
		if (outcome.missing) {
			console.warn(`[${label}] path not found, skipping: ${outcome.file}`);

			continue;
		}

		if (!outcome.changed) {
			continue;
		}

		changedCount++;
		console.log(`[${label}] ${check ? 'would change' : 'updated'} ${outcome.file}`);
	}

	if (check && changedCount > 0) {
		console.error(`[${label}] ${changedCount} file(s) need edits. Run "pnpm format" to fix.`);
		process.exit(1);
	}

	console.log(`[${label}] processed ${files.length} file(s) in ${process.cwd()}, ${changedCount} ${check ? 'would change' : 'changed'}`);
}

export function runOxfmt(options: CliOptions): Promise<void> {
	if (!options.oxfmtBin || options.formatFiles.length === 0) {
		return Promise.resolve();
	}

	const args = options.oxfmtConfig ? ['--config', options.oxfmtConfig] : [];

	args.push(options.check ? '--check' : '--write', '--no-error-on-unmatched-pattern');

	const bin = options.oxfmtBin;
	const files = options.formatFiles;

	const runChunk = (chunk: string[]): Promise<void> => {
		return new Promise((resolvePromise, rejectPromise) => {
			const child = spawn(bin, [...args, ...chunk], { stdio: 'inherit' });

			child.on('error', rejectPromise);

			child.on('exit', (code, signal) => {
				if (code === 0) {
					resolvePromise();
				} else {
					rejectPromise(new Error(`[format-all] oxfmt exited with ${signal ?? code}`));
				}
			});
		});
	};

	let promise = Promise.resolve();

	for (let i = 0; i < files.length; i += OXFMT_CHUNK_SIZE) {
		const chunk = files.slice(i, i + OXFMT_CHUNK_SIZE);

		promise = promise.then(() => {
			return runChunk(chunk);
		});
	}

	return promise;
}

async function runValidate(files: string[]): Promise<void> {
	const failures = (await mapPool(files, availableParallelism(), validateFile)).flat();

	if (failures.length > 0) {
		console.error(failures.join('\n'));
		console.error(`[validate-syntax] ${failures.length} syntax error(s) found after formatting.`);
		process.exit(1);
	}

	console.log(`[validate-syntax] checked ${files.length} file(s) in ${process.cwd()}`);
}

export async function main(): Promise<void> {
	const options = parseArgs(process.argv.slice(2));
	const formatTargets = [...new Set(options.formatFiles.filter(isTargetFile))];

	const syntaxTargets = [
		...new Set(
			options.syntaxFiles.filter((file) => {
				return file.endsWith('.ts') || file.endsWith('.vue');
			}),
		),
	];

	const pipelineOptions = { ...options, formatFiles: formatTargets };

	await runPass('blank-lines', formatTargets, options.check, processBlankLinesFile);

	await runOxfmt(pipelineOptions);

	await runPass('fluent-chains', formatTargets, options.check, processFluentChainsFile);

	await runOxfmt(pipelineOptions);

	await runValidate(syntaxTargets);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	main().catch((err: unknown) => {
		console.error(err);
		process.exit(1);
	});
}
