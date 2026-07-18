import { availableParallelism } from 'node:os';
import type { OxfmtRunFailed, SourceFileUnreadable, SourceUnparsable } from '#sidecar/errors';
import { FluentChains } from '#sidecar/fluent-chains';
import type { ProcessRunner } from '#sidecar/process-runner';
import { isErr, ok } from '#sidecar/result';
import type { Result } from '#sidecar/result';
import { processSegment } from '#sidecar/segment';
import type { SourceFileError, SourceFiles } from '#sidecar/source-files';
import { Sources } from '#sidecar/sources';
import { VueScript } from '#sidecar/vue-script';

const OXFMT_CHUNK_SIZE = 100;

/** Whether a pipeline pass checks source or writes its computed changes. */
export type FormatMode = 'check' | 'write';

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

/** A source file that could not be read or parsed during validation. */
export type ValidationFailure = {
	/** The original source path reported to the user. */
	readonly file: string;

	/** The carried read or parse failure. */
	readonly error: SourceFileUnreadable | SourceUnparsable;
};

type ProcessOne = (file: string, mode: FormatMode) => Promise<Result<boolean, SourceFileError>>;

async function mapPool<T, R>(items: T[], limit: number, fn: (item: T) => Promise<R>): Promise<R[]> {
	const results = new Array<R>(items.length);

	let nextIndex = 0;

	async function worker(): Promise<void> {
		while (true) {
			const index = nextIndex++;

			if (index >= items.length) {
				return;
			}

			const item = items[index];

			if (item === undefined) {
				continue;
			}

			results[index] = await fn(item);
		}
	}

	const workers = Array.from({ length: Math.max(1, Math.min(limit, items.length)) }, worker);

	await Promise.all(workers);

	return results;
}

function scriptExtension(openingTag: string): 'ts' | 'tsx' {
	const lang = VueScript.attribute(openingTag, 'lang') ?? '';

	return lang === 'tsx' || lang === 'jsx' ? 'tsx' : 'ts';
}

function scriptPrefix(content: string, scriptStart: number): string {
	return content.slice(0, scriptStart).replace(/[^\r\n]/g, ' ');
}

/** Coordinates formatting and validation through narrow filesystem and process ports. */
export class FormatPipeline {
	readonly #sourceFiles: SourceFiles;
	readonly #processRunner: ProcessRunner;

	/**
	 * @param dependencies - The filesystem and process ports used by the pipeline.
	 * @param dependencies.sourceFiles - Reads and atomically writes source files.
	 * @param dependencies.processRunner - Invokes oxfmt with inherited standard streams.
	 */
	constructor(dependencies: { sourceFiles: SourceFiles; processRunner: ProcessRunner }) {
		this.#sourceFiles = dependencies.sourceFiles;
		this.#processRunner = dependencies.processRunner;
	}

	/**
	 * Apply the blank-line formatting pass to one TypeScript or Vue file.
	 *
	 * @param path - The source file to format.
	 * @param mode - Whether to check or atomically write changes.
	 * @returns Whether the file changes, or the typed filesystem failure.
	 */
	async formatFile(path: string, mode: FormatMode): Promise<Result<boolean, SourceFileError>> {
		const read = await this.#sourceFiles.readText(path);

		if (isErr(read)) {
			return read;
		}

		const original = read.value;

		let updated = original;

		if (path.endsWith('.vue')) {
			const segments = VueScript.extractBlocks(original).filter((segment) => {
				return VueScript.isJavaScriptOrTypeScript(segment.openTag);
			});

			for (const segment of [...segments].reverse()) {
				const rewritten = processSegment(segment.content, `${path}.script.ts`);

				if (rewritten === segment.content) {
					continue;
				}

				updated = updated.slice(0, segment.start) + rewritten + updated.slice(segment.start + segment.content.length);
			}
		} else {
			updated = processSegment(original, path);
		}

		return this.#writeChanged(path, original, updated, mode);
	}

	/**
	 * Apply fluent-chain formatting to one TypeScript or Vue file.
	 *
	 * @param path - The source file to format.
	 * @param mode - Whether to check or atomically write changes.
	 * @returns Whether the file changes, or the typed filesystem failure.
	 */
	formatFluentFile(path: string, mode: FormatMode): Promise<Result<boolean, SourceFileError>> {
		return FluentChains.formatFile(path, mode, this.#sourceFiles);
	}

	/**
	 * Process files concurrently while preserving outcome order.
	 *
	 * @param label - The pass label associated with the outcomes.
	 * @param files - The source paths to process.
	 * @param mode - Whether the pass checks or writes changes.
	 * @param processOne - The operation applied to each source path.
	 * @returns One effect-free reporting outcome per input path.
	 */
	async runPass(label: string, files: string[], mode: FormatMode, processOne: ProcessOne): Promise<PassOutcome[]> {
		return mapPool(
			files,
			availableParallelism(),
			async (file): Promise<PassOutcome> => {
				const outcome = await processOne(file, mode);

				if (isErr(outcome)) {
					return { label, file, changed: false, error: outcome.error };
				}

				return { label, file, changed: outcome.value, error: null };
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
	 * Validate TypeScript files and JavaScript-compatible Vue script blocks.
	 *
	 * @param files - The source paths to validate.
	 * @returns Carried read and parse failures in deterministic input order.
	 */
	async validate(files: string[]): Promise<ValidationFailure[]> {
		const failures = await mapPool(
			files,
			availableParallelism(),
			async (file): Promise<ValidationFailure[]> => {
				const read = await this.#sourceFiles.readText(file);

				if (isErr(read)) {
					return [{ file, error: read.error }];
				}

				if (!file.endsWith('.vue')) {
					const parsed = Sources.parse(file, read.value);

					return isErr(parsed) ? [{ file, error: parsed.error }] : [];
				}

				const vueFailures: ValidationFailure[] = [];

				for (const block of VueScript.extractBlocks(read.value)) {
					if (!VueScript.isJavaScriptOrTypeScript(block.openTag)) {
						continue;
					}

					const virtualContent = scriptPrefix(read.value, block.start) + block.content;
					const parsed = Sources.parse(`${file}.script.${scriptExtension(block.openTag)}`, virtualContent);

					if (isErr(parsed)) {
						vueFailures.push({ file, error: parsed.error });
					}
				}

				return vueFailures;
			},
		);

		return failures.flat();
	}

	async #writeChanged(path: string, original: string, updated: string, mode: FormatMode): Promise<Result<boolean, SourceFileError>> {
		if (updated === original) {
			return ok(false);
		}

		if (mode === 'write') {
			const written = await this.#sourceFiles.writeTextAtomic(path, updated);

			if (isErr(written)) {
				return written;
			}
		}

		return ok(true);
	}
}
