import { z } from 'zod';
import { UnexpectedCliArgument } from '#sidecar/kernel/errors';
import type { FormatMode } from '#sidecar/pipeline/format-pipeline';
import { err, ok } from '#sidecar/kernel/result';
import type { Result } from '#sidecar/kernel/result';

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
