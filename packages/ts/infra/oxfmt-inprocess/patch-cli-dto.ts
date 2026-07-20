import { z } from 'zod';

/** Immutable command-line options for the oxfmt in-process patch entrypoint. */
export class PatchCliDto {
	/** Directory containing the oxfmt distribution to patch. */
	readonly distDir: string;

	static readonly #argvSchema = z.array(z.string());

	static readonly #schema = z.object({
		distDir: z.string().min(1),
	});

	private constructor(value: { distDir: string }) {
		this.distDir = value.distDir;

		Object.setPrototypeOf(this, Object.prototype);
		Object.freeze(this);
	}

	/**
	 * Parse the oxfmt patch command line.
	 *
	 * @param input - Arguments after the executable and script path.
	 * @returns Immutable patch options, or `null` when the distribution path is absent.
	 */
	static parse(input: unknown): PatchCliDto | null {
		const argv = PatchCliDto.#argvSchema.parse(input);
		const parsed = PatchCliDto.#schema.safeParse({ distDir: argv[0] });

		return parsed.success ? new PatchCliDto(parsed.data) : null;
	}
}
