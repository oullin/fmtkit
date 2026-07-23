import { availableParallelism } from 'node:os';
import type { EmbeddedBlockSplitter } from '#sidecar/hosts/embedded-block-splitter';
import type { SourceFileUnreadable, SourceUnparsable } from '#sidecar/kernel/errors';
import { isErr } from '#sidecar/kernel/result';
import { mapPool } from '#sidecar/kernel/concurrency';
import type { SourceFiles } from '#sidecar/io/source-files';
import type { SourceParser } from '#sidecar/syntax/source-parser';

/** A source file that could not be read or parsed during validation. */
export type ValidationFailure = {
	/** The original source path reported to the user. */
	readonly file: string;

	/** The carried read or parse failure. */
	readonly error: SourceFileUnreadable | SourceUnparsable;
};

/** Validates TypeScript files and the JavaScript-compatible blocks of host documents. */
export class SyntaxValidator {
	readonly #sourceFiles: SourceFiles;
	readonly #splitter: EmbeddedBlockSplitter;
	readonly #parser: SourceParser;

	static #scriptPrefix(content: string, scriptStart: number): string {
		return content.slice(0, scriptStart).replace(/[^\r\n]/g, ' ');
	}

	/**
	 * @param dependencies - The filesystem port and syntax services used to validate.
	 * @param dependencies.sourceFiles - Reads source files for parsing.
	 * @param dependencies.splitter - Extracts host embedded blocks.
	 * @param dependencies.parser - Parses source and reports syntax failures.
	 */
	constructor(dependencies: { sourceFiles: SourceFiles; splitter: EmbeddedBlockSplitter; parser: SourceParser }) {
		this.#sourceFiles = dependencies.sourceFiles;
		this.#splitter = dependencies.splitter;
		this.#parser = dependencies.parser;
	}

	/**
	 * Validate TypeScript files and JavaScript-compatible embedded host blocks.
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

				if (!this.#splitter.isHost(file)) {
					const parsed = this.#parser.parse(file, read.value);

					return isErr(parsed) ? [{ file, error: parsed.error }] : [];
				}

				const hostFailures: ValidationFailure[] = [];

				for (const block of this.#splitter.extract(file, read.value)) {
					const virtualContent = SyntaxValidator.#scriptPrefix(read.value, block.start) + block.content;
					const parsed = this.#parser.parse(`${file}.script.${block.extension}`, virtualContent);

					if (isErr(parsed) && this.#splitter.hardValidated(file)) {
						hostFailures.push({ file, error: parsed.error });
					}
				}

				return hostFailures;
			},
		);

		return failures.flat();
	}
}
