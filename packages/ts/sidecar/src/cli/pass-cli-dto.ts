import { z } from 'zod';
import type { FileTargetPolicy } from '#sidecar/hosts/file-target-policy';

/** Immutable command-line options shared by standalone formatting passes. */
export class PassCliDto {
	/** Whether the pass checks source or writes changes. */
	readonly mode: 'check' | 'write';

	/** Files eligible for formatting. */
	readonly files: readonly string[];

	static readonly #argvSchema = z.array(z.string());

	static readonly #schema = z.object({
		mode: z.enum(['check', 'write']),
		files: z.array(z.string()),
	});

	private constructor(value: { mode: 'check' | 'write'; files: string[] }) {
		this.mode = value.mode;
		this.files = Object.freeze(value.files);

		Object.setPrototypeOf(this, Object.prototype);
		Object.freeze(this);
	}

	/**
	 * Parse a standalone formatting pass command line.
	 *
	 * @param input - Arguments after the executable and script path.
	 * @param targets - The policy that classifies eligible target files.
	 * @returns Immutable formatting pass options.
	 */
	static parse(input: unknown, targets: FileTargetPolicy): PassCliDto {
		const argv = PassCliDto.#argvSchema.parse(input);

		const candidate = {
			mode: argv.includes('--check') ? ('check' as const) : ('write' as const),
			files: argv.filter((argument) => {
				return argument !== '--check' && targets.isTargetFile(argument);
			}),
		};

		return new PassCliDto(PassCliDto.#schema.parse(candidate));
	}
}
