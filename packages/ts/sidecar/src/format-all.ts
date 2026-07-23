import { pathToFileURL } from 'node:url';
import { z } from 'zod';
import { UnexpectedCliArgument } from '#sidecar/kernel/errors';
import type { OxcErrorDto } from '#sidecar/kernel/errors';
import { FileTargets } from '#sidecar/hosts/file-targets';
import { FormatPipeline } from '#sidecar/pipeline/format-pipeline';
import type { FormatMode, PassOutcome, ValidationFailure } from '#sidecar/pipeline/format-pipeline';
import { NodeProcessRunner } from '#sidecar/io/process-runner';
import { err, isErr, ok } from '#sidecar/kernel/result';
import type { Result } from '#sidecar/kernel/result';
import { NodeSourceFiles } from '#sidecar/io/source-files';
import { PipelineFactory } from '#sidecar/pipeline/pipeline-factory';
import { SourceFileEditor } from '#sidecar/pipeline/source-file-editor';

/** Immutable command-line options for the full formatting pipeline. */
export class CliOptionsDto {
	/** Whether the pipeline checks source or writes changes. */
	readonly mode: FormatMode;

	/** The oxfmt executable, or `null` to skip external formatting. */
	readonly oxfmtBin: string | null;

	/** The oxfmt configuration path, or `null` to use its defaults. */
	readonly oxfmtConfig: string | null;

	/** Files eligible for formatting passes. */
	readonly formatFiles: readonly string[];

	/** Files eligible for final syntax validation. */
	readonly syntaxFiles: readonly string[];

	static readonly #argvSchema = z.array(z.string());

	static readonly #schema = z.object({
		mode: z.enum(['check', 'write']),
		oxfmtBin: z.string().nullable(),
		oxfmtConfig: z.string().nullable(),
		formatFiles: z.array(z.string()),
		syntaxFiles: z.array(z.string()),
	});

	private constructor(value: { mode: FormatMode; oxfmtBin: string | null; oxfmtConfig: string | null; formatFiles: string[]; syntaxFiles: string[] }) {
		this.mode = value.mode;
		this.oxfmtBin = value.oxfmtBin;
		this.oxfmtConfig = value.oxfmtConfig;
		this.formatFiles = Object.freeze(value.formatFiles);
		this.syntaxFiles = Object.freeze(value.syntaxFiles);

		Object.setPrototypeOf(this, Object.prototype);
		Object.freeze(this);
	}

	/**
	 * Parse the full-pipeline command line.
	 *
	 * @param input - Arguments after the executable and script path.
	 * @returns Parsed options, or the unexpected argument as a typed value.
	 */
	static parse(input: unknown): Result<CliOptionsDto, UnexpectedCliArgument> {
		const argv = CliOptionsDto.#argvSchema.parse(input);

		const candidate = {
			mode: 'write' as FormatMode,
			oxfmtBin: null as string | null,
			oxfmtConfig: null as string | null,
			formatFiles: [] as string[],
			syntaxFiles: [] as string[],
		};

		let section: 'formatFiles' | 'syntaxFiles' | null = null;

		for (let index = 0; index < argv.length; index++) {
			const argument = argv[index];

			if (argument === undefined) {
				continue;
			}

			if (argument === '--check') {
				candidate.mode = 'check';
				section = null;
			} else if (argument === '--oxfmt-bin') {
				candidate.oxfmtBin = argv[++index] ?? null;
				section = null;
			} else if (argument === '--oxfmt-config') {
				candidate.oxfmtConfig = argv[++index] ?? null;
				section = null;
			} else if (argument === '--format-files') {
				section = 'formatFiles';
			} else if (argument === '--syntax-files') {
				section = 'syntaxFiles';
			} else if (section) {
				candidate[section].push(argument);
			} else {
				return err(new UnexpectedCliArgument(argument));
			}
		}

		return ok(new CliOptionsDto(CliOptionsDto.#schema.parse(candidate)));
	}
}

/** Reports pipeline values without coupling formatting passes to the console. */
class FormatAllReporter {
	/**
	 * Report one formatting pass and decide whether execution may continue.
	 *
	 * @param label - The formatting pass label.
	 * @param files - The source paths requested for the pass.
	 * @param mode - Whether the pass checked or wrote source.
	 * @param outcomes - The ordered outcomes produced by the pass.
	 * @param failureNoun - The change description used in check-mode guidance.
	 * @returns `true` when no outcome or pending change makes the pass fail.
	 */
	static reportPass(label: string, files: readonly string[], mode: FormatMode, outcomes: PassOutcome[], failureNoun: string): boolean {
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

			if (outcome.changed) {
				changedCount++;
				console.log(`[${label}] ${mode === 'check' ? 'would change' : 'updated'} ${outcome.file}`);
			}
		}

		if (mode === 'check' && changedCount > 0) {
			console.error(`[${label}] ${changedCount} file(s) need ${failureNoun}. Run "pnpm format" to fix.`);

			return false;
		}

		console.log(`[${label}] processed ${files.length} file(s) in ${process.cwd()}, ${changedCount} ${mode === 'check' ? 'would change' : 'changed'}`);

		return true;
	}

	/**
	 * Format one parser diagnostic for console output.
	 *
	 * @param file - The source path associated with the diagnostic.
	 * @param error - The parser diagnostic to render.
	 * @returns A source-framed message, plain message, or stable fallback.
	 */
	static formatError(file: string, error: OxcErrorDto): string {
		if (error.codeframe && error.codeframe.length > 0) {
			return `[validate-syntax] ${file}\n${error.codeframe.trimEnd()}`;
		}

		if (error.message && error.message.length > 0) {
			return `[validate-syntax] ${file}: ${error.message}`;
		}

		return `[validate-syntax] ${file}: syntax validation failed`;
	}

	/**
	 * Report syntax-validation failures and decide whether execution succeeded.
	 *
	 * @param files - The source paths requested for validation.
	 * @param failures - The ordered read and parse failures.
	 * @returns `true` when no reportable validation failure remains.
	 */
	static reportValidation(files: readonly string[], failures: ValidationFailure[]): boolean {
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
				diagnostics.push(FormatAllReporter.formatError(failure.file, error));
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
}

/**
 * Run the full formatting CLI and map outcome values to console output and status.
 *
 * @returns Nothing after reporting outcomes and setting the process status.
 */
export async function main(): Promise<void> {
	const parsed = CliOptionsDto.parse(process.argv.slice(2));

	if (isErr(parsed)) {
		console.error(parsed.error);
		process.exitCode = 1;

		return;
	}

	const options = parsed.value;
	const formatTargets = [...new Set(options.formatFiles.filter(FileTargets.isTargetFile))];
	const syntaxTargets = [...new Set(options.syntaxFiles.filter(FileTargets.isSyntaxTarget))];
	const factory = PipelineFactory.create();
	const sourceFiles = new NodeSourceFiles();

	const pipeline = new FormatPipeline({
		editor: new SourceFileEditor({ sourceFiles }),
		processRunner: new NodeProcessRunner(),
		validator: factory.syntaxValidator(sourceFiles),
	});

	const segmentFormatter = factory.segmentFormatter();

	const blankLines = await pipeline.runPass(segmentFormatter, formatTargets, options.mode);

	if (!FormatAllReporter.reportPass('blank-lines', formatTargets, options.mode, blankLines, 'edits')) {
		process.exitCode = 1;

		return;
	}

	const oxfmt = await pipeline.runOxfmt({ bin: options.oxfmtBin, config: options.oxfmtConfig, files: formatTargets, mode: options.mode });

	if (isErr(oxfmt)) {
		console.error(oxfmt.error);
		process.exitCode = 1;

		return;
	}

	const fluentChains = await pipeline.runPass(factory.fluentFormatter(), formatTargets, options.mode);

	if (!FormatAllReporter.reportPass('fluent-chains', formatTargets, options.mode, fluentChains, 'edits')) {
		process.exitCode = 1;

		return;
	}

	// Fluent and expanded calls create blank-line obligations the first pass
	// cannot see, so the second pass makes one invocation reach a fixed point.
	const finalBlankLines = await pipeline.runPass(segmentFormatter, formatTargets, options.mode);

	if (!FormatAllReporter.reportPass('blank-lines', formatTargets, options.mode, finalBlankLines, 'edits')) {
		process.exitCode = 1;

		return;
	}

	if (!FormatAllReporter.reportValidation(syntaxTargets, await pipeline.validate(syntaxTargets))) {
		process.exitCode = 1;
	}
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	main().catch((error: unknown) => {
		console.error(error);
		process.exitCode = 1;
	});
}
