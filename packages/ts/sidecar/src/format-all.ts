import { pathToFileURL } from 'node:url';
import type { OxcError } from 'oxc-parser';
import { UnexpectedCliArgument } from '#sidecar/errors';
import { isTargetFile } from '#sidecar/file-targets';
import { FormatPipeline } from '#sidecar/format-pipeline';
import type { FormatMode, PassOutcome, ValidationFailure } from '#sidecar/format-pipeline';
import { NodeProcessRunner } from '#sidecar/process-runner';
import { err, isErr, ok } from '#sidecar/result';
import type { Result } from '#sidecar/result';
import { NodeSourceFiles } from '#sidecar/source-files';

/** Parsed command-line options for the full formatting pipeline. */
export type CliOptions = {
	/** Whether the pipeline checks source or writes changes. */
	mode: FormatMode;

	/** The oxfmt executable, or `null` to skip external formatting. */
	oxfmtBin: string | null;

	/** The oxfmt configuration path, or `null` to use its defaults. */
	oxfmtConfig: string | null;

	/** Files eligible for formatting passes. */
	readonly formatFiles: string[];

	/** Files eligible for final syntax validation. */
	readonly syntaxFiles: string[];
};

/**
 * Parse the full-pipeline command line.
 *
 * @param argv - Arguments after the executable and script path.
 * @returns Parsed options, or the unexpected argument as a typed value.
 */
export function parseArgs(argv: string[]): Result<CliOptions, UnexpectedCliArgument> {
	const options: CliOptions = {
		mode: 'write',
		oxfmtBin: null,
		oxfmtConfig: null,
		formatFiles: [],
		syntaxFiles: [],
	};

	let section: 'formatFiles' | 'syntaxFiles' | null = null;

	for (let i = 0; i < argv.length; i++) {
		const arg = argv[i];

		if (arg === undefined) {
			continue;
		}

		if (arg === '--check') {
			options.mode = 'check';
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
			return err(new UnexpectedCliArgument(arg));
		}
	}

	return ok(options);
}

function reportPass(label: string, files: string[], mode: FormatMode, outcomes: PassOutcome[], failureNoun: string): boolean {
	let changedCount = 0;

	for (const outcome of outcomes) {
		if (outcome.error?._tag === 'SourceFileUnreadable' && outcome.error.isNotFound()) {
			console.warn(`[${label}] path not found, skipping: ${outcome.file}`);

			continue;
		}

		if (outcome.error) {
			console.error(outcome.error);

			return false;
		}

		if (!outcome.changed) {
			continue;
		}

		changedCount++;
		console.log(`[${label}] ${mode === 'check' ? 'would change' : 'updated'} ${outcome.file}`);
	}

	if (mode === 'check' && changedCount > 0) {
		console.error(`[${label}] ${changedCount} file(s) need ${failureNoun}. Run "pnpm format" to fix.`);

		return false;
	}

	console.log(`[${label}] processed ${files.length} file(s) in ${process.cwd()}, ${changedCount} ${mode === 'check' ? 'would change' : 'changed'}`);

	return true;
}

function formatError(file: string, error: OxcError): string {
	if (typeof error.codeframe === 'string' && error.codeframe.length > 0) {
		return `[validate-syntax] ${file}\n${error.codeframe.trimEnd()}`;
	}

	if (typeof error.message === 'string' && error.message.length > 0) {
		return `[validate-syntax] ${file}: ${error.message}`;
	}

	return `[validate-syntax] ${file}: syntax validation failed`;
}

function reportValidation(files: string[], failures: ValidationFailure[]): boolean {
	const diagnostics: string[] = [];

	for (const failure of failures) {
		if (failure.error._tag === 'SourceFileUnreadable') {
			if (failure.error.isNotFound()) {
				console.warn(`[validate-syntax] path not found, skipping: ${failure.file}`);

				continue;
			}

			console.error(failure.error);

			return false;
		}

		for (const error of failure.error.errors) {
			diagnostics.push(formatError(failure.file, error));
		}
	}

	if (diagnostics.length > 0) {
		console.error(diagnostics.join('\n'));
		console.error(`[validate-syntax] ${diagnostics.length} syntax error(s) found after formatting.`);

		return false;
	}

	console.log(`[validate-syntax] checked ${files.length} file(s) in ${process.cwd()}`);

	return true;
}

/**
 * Run the full formatting CLI and map outcome values to console output and status.
 *
 * @returns Nothing after setting `process.exitCode` when the pipeline fails.
 */
export async function main(): Promise<void> {
	const parsed = parseArgs(
		process.argv.slice(2),
	);

	if (isErr(parsed)) {
		console.error(parsed.error);
		process.exitCode = 1;

		return;
	}

	const options = parsed.value;
	const formatTargets = [...new Set(options.formatFiles.filter(isTargetFile))];

	const syntaxTargets = [
		...new Set(
			options.syntaxFiles.filter((file) => {
				return file.endsWith('.ts') || file.endsWith('.vue');
			}),
		),
	];

	const pipeline = new FormatPipeline({ sourceFiles: new NodeSourceFiles(), processRunner: new NodeProcessRunner() });

	const blankLines = await pipeline.runPass('blank-lines', formatTargets, options.mode, (file, mode) => {
		return pipeline.formatFile(file, mode);
	});

	if (!reportPass('blank-lines', formatTargets, options.mode, blankLines, 'edits')) {
		process.exitCode = 1;

		return;
	}

	const oxfmt = await pipeline.runOxfmt({ bin: options.oxfmtBin, config: options.oxfmtConfig, files: formatTargets, mode: options.mode });

	if (isErr(oxfmt)) {
		console.error(oxfmt.error);
		process.exitCode = 1;

		return;
	}

	const fluentChains = await pipeline.runPass('fluent-chains', formatTargets, options.mode, (file, mode) => {
		return pipeline.formatFluentFile(file, mode);
	});

	if (!reportPass('fluent-chains', formatTargets, options.mode, fluentChains, 'edits')) {
		process.exitCode = 1;

		return;
	}

	// Fluent and expanded calls create blank-line obligations the first pass
	// cannot see, so the second pass makes one invocation reach a fixed point.
	const finalBlankLines = await pipeline.runPass('blank-lines', formatTargets, options.mode, (file, mode) => {
		return pipeline.formatFile(file, mode);
	});

	if (!reportPass('blank-lines', formatTargets, options.mode, finalBlankLines, 'edits')) {
		process.exitCode = 1;

		return;
	}

	if (!reportValidation(syntaxTargets, await pipeline.validate(syntaxTargets))) {
		process.exitCode = 1;
	}
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	main().catch((error: unknown) => {
		console.error(error);
		process.exitCode = 1;
	});
}
