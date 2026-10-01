import { availableParallelism } from 'node:os';
import type { EmbeddedBlockSplitter } from '#sidecar/hosts/embedded-block-splitter';
import type { SourceFileUnreadable, SourceUnparsable } from '#sidecar/kernel/errors';
import { isErr } from '#sidecar/kernel/result';
import { mapPool } from '#sidecar/kernel/concurrency';
import type { SourceFiles } from '#sidecar/io/source-files';
import type { SourceParser } from '#sidecar/syntax/source-parser';

/** A source file that could not be read or parsed during validation. */
export type ValidationFailure = {
	/** The carried read or parse failure. */
	readonly error: SourceFileUnreadable | SourceUnparsable;

	/** The original source path reported to the user. */
	readonly file: string;
};

/** Validates TypeScript files and the JavaScript-compatible blocks of host documents. */
export class SyntaxValidator {
	readonly #sourceFiles: SourceFiles;
	readonly #splitter: EmbeddedBlockSplitter;
	readonly #parser: SourceParser;

	static #scriptPrefix(content: string, scriptStart: number): string {
		return content.slice(0, scriptStart).replaceAll(/[^\r\n]/g, ' ');
	}

	/**
	 * @param dependencies - The filesystem port and syntax services used to validate.
	 * @param dependencies.sourceFiles - Reads source files for parsing.
	 * @param dependencies.splitter - Extracts host embedded blocks.
	 * @param dependencies.parser - Parses source and reports syntax failures.
	 */
	constructor(dependencies: { parser: SourceParser; sourceFiles: SourceFiles; splitter: EmbeddedBlockSplitter }) {
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
	async validate(files: Array<string>): Promise<Array<ValidationFailure>> {
		const failures = await mapPool(
			files,
			availableParallelism(),
			async (file): Promise<Array<ValidationFailure>> => {
				const read = await this.#sourceFiles.readText(file);

				if (isErr(read)) {
					return [{ error: read.error, file }];
				}

				if (!this.#splitter.isHost(file)) {
					const parsed = this.#parser.parse(file, read.value);

					return isErr(parsed) ? [{ error: parsed.error, file }] : [];
				}

				const hostFailures: Array<ValidationFailure> = [];

				for (const block of this.#splitter.extract(file, read.value)) {
					const virtualContent = SyntaxValidator.#scriptPrefix(read.value, block.start) + block.content;
					const parsed = this.#parser.parse(`${file}.script.${block.extension}`, virtualContent);

					if (isErr(parsed) && this.#splitter.hardValidated(file)) {
						hostFailures.push({ error: parsed.error, file });
					}
				}

				return hostFailures;
			},
		);

		return failures.flat();
	}
}
