import { z } from 'zod';

/** Immutable command-line options for standalone syntax validation. */
export class SyntaxCliDto {
	/** TypeScript and Vue files eligible for syntax validation. */
	readonly files: readonly string[];

	static readonly #argvSchema = z.array(z.string());

	static readonly #schema = z.object({
		files: z.array(z.string()),
	});

	private constructor(value: { files: string[] }) {
		this.files = Object.freeze(value.files);

		Object.setPrototypeOf(this, Object.prototype);
		Object.freeze(this);
	}

	/**
	 * Parse the standalone syntax-validation command line.
	 *
	 * @param input - Arguments after the executable and script path.
	 * @returns Immutable syntax-validation options.
	 */
	static parse(input: unknown): SyntaxCliDto {
		const argv = SyntaxCliDto.#argvSchema.parse(input);

		const files = argv.filter((file) => {
			return file.endsWith('.ts') || file.endsWith('.vue');
		});

		return new SyntaxCliDto(SyntaxCliDto.#schema.parse({ files }));
	}
}
