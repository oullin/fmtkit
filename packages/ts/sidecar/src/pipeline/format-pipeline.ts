import { availableParallelism } from 'node:os';
import type { OxfmtRunFailed } from '#sidecar/kernel/errors';
import type { FileFormatter } from '#sidecar/pipeline/file-formatter';
import { mapPool } from '#sidecar/kernel/concurrency';
import type { ProcessRunner } from '#sidecar/io/process-runner';
import { isErr, ok } from '#sidecar/kernel/result';
import type { Result } from '#sidecar/kernel/result';
import type { SourceFileEditor } from '#sidecar/pipeline/source-file-editor';
import type { SourceFileError } from '#sidecar/io/source-files';
import type { SyntaxValidator, ValidationFailure } from '#sidecar/pipeline/syntax-validator';

const OXFMT_CHUNK_SIZE = 100;

/** Whether a pipeline pass checks source or writes its computed changes. */
export type FormatMode = 'check' | 'write';

export type { ValidationFailure } from '#sidecar/pipeline/syntax-validator';

/** The result of processing one file in a formatting pass. */
export type PassOutcome = {
	/** The formatting pass that produced the outcome. */
	readonly label: string;

	/** The requested source path. */
	readonly file: string;

	/** Whether the pass would change or did change the source. */
	readonly changed: boolean;

	/** The typed filesystem failure, or `null` when processing completed. */
	readonly error: SourceFileError | null;
};

/** The options needed to invoke oxfmt over one pipeline stage. */
export type OxfmtOptions = {
	/** The executable to invoke, or `null` to skip the stage. */
	readonly bin: string | null;

	/** The oxfmt configuration path, or `null` to use defaults. */
	readonly config: string | null;

	/** The source paths passed to oxfmt. */
	readonly files: string[];

	/** Whether oxfmt checks source or writes changes. */
	readonly mode: FormatMode;
};

/** Coordinates formatting and validation through narrow filesystem and process ports. */
export class FormatPipeline {
	readonly #editor: SourceFileEditor;
	readonly #processRunner: ProcessRunner;
	readonly #validator: SyntaxValidator;

	/**
	 * @param dependencies - The editor, process port, and validator used by the pipeline.
	 * @param dependencies.editor - Reads, transforms, and atomically writes single files.
	 * @param dependencies.processRunner - Invokes oxfmt with inherited standard streams.
	 * @param dependencies.validator - Validates formatted source and host blocks.
	 */
	constructor(dependencies: { editor: SourceFileEditor; processRunner: ProcessRunner; validator: SyntaxValidator }) {
		this.#editor = dependencies.editor;
		this.#processRunner = dependencies.processRunner;
		this.#validator = dependencies.validator;
	}

	/**
	 * Run a file formatter over every path concurrently, preserving outcome order.
	 *
	 * @param formatter - The file formatter whose pipeline and label drive the pass.
	 * @param files - The source paths to process.
	 * @param mode - Whether the pass checks or writes changes.
	 * @returns One effect-free reporting outcome per input path.
	 */
	async runPass(formatter: FileFormatter, files: string[], mode: FormatMode): Promise<PassOutcome[]> {
		return mapPool(
			files,
			availableParallelism(),
			async (file): Promise<PassOutcome> => {
				const outcome = await this.#editor.apply(file, mode, (content) => {
					return formatter.format(file, content);
				});

				if (isErr(outcome)) {
					return { label: formatter.label, file, changed: false, error: outcome.error };
				}

				return { label: formatter.label, file, changed: outcome.value, error: null };
			},
		);
	}

	/**
	 * Run oxfmt sequentially over bounded file chunks.
	 *
	 * @param options - The executable, configuration, files, and format mode.
	 * @returns Nothing, or the first typed oxfmt failure.
	 */
	async runOxfmt(options: OxfmtOptions): Promise<Result<void, OxfmtRunFailed>> {
		if (!options.bin || options.files.length === 0) {
			return ok(undefined);
		}

		const args = options.config ? ['--config', options.config] : [];

		args.push(options.mode === 'check' ? '--check' : '--write', '--no-error-on-unmatched-pattern');

		for (let i = 0; i < options.files.length; i += OXFMT_CHUNK_SIZE) {
			const outcome = await this.#processRunner.run(options.bin, [...args, ...options.files.slice(i, i + OXFMT_CHUNK_SIZE)]);

			if (isErr(outcome)) {
				return outcome;
			}
		}

		return ok(undefined);
	}

	/**
	 * Validate TypeScript files and JavaScript-compatible embedded host blocks.
	 *
	 * @param files - The source paths to validate.
	 * @returns Carried read and parse failures in deterministic input order.
	 */
	async validate(files: string[]): Promise<ValidationFailure[]> {
		return this.#validator.validate(files);
	}
}
