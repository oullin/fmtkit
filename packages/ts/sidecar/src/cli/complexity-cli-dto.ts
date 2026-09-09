import { z } from 'zod';

const SCORABLE_SUFFIXES = ['.ts', '.tsx', '.mts', '.cts', '.js', '.jsx'];

const DECLARATION_SUFFIXES = ['.d.ts', '.d.mts', '.d.cts'];

const TEST_INFIXES = ['.test.', '.spec.'];

/** Immutable command-line options for the complexity scan. */
export class ComplexityCliDto {
	/** The directory the reported file paths are made relative to. */
	readonly root: string;

	/** A NUL-separated list file naming the sources to scan, or `''` for none. */
	readonly filesFrom: string;

	/** Sources named directly on the command line. */
	readonly files: readonly string[];

	static readonly #argvSchema = z.array(z.string());

	static readonly #schema = z.object({
		root: z.string(),
		filesFrom: z.string(),
		files: z.array(z.string()),
	});

	private constructor(value: { root: string; filesFrom: string; files: string[] }) {
		this.root = value.root;
		this.filesFrom = value.filesFrom;
		this.files = Object.freeze(value.files);

		Object.setPrototypeOf(this, Object.prototype);
		Object.freeze(this);
	}

	/**
	 * Report whether a path is one the complexity scan can score.
	 *
	 * @param path - The path to classify.
	 * @returns `true` for the TypeScript and JavaScript families, tests and declarations aside.
	 */
	static isScorable(path: string): boolean {
		if (DECLARATION_SUFFIXES.some((suffix) => path.endsWith(suffix))) {
			return false;
		}

		if (TEST_INFIXES.some((infix) => path.includes(infix))) {
			return false;
		}

		return SCORABLE_SUFFIXES.some((suffix) => path.endsWith(suffix));
	}

	/**
	 * Parse the complexity command line.
	 *
	 * @param input - Arguments after the executable and script path.
	 * @returns Immutable scan options.
	 */
	static parse(input: readonly string[]): ComplexityCliDto {
		const argv = ComplexityCliDto.#argvSchema.parse(input);

		const candidate = { root: process.cwd(), filesFrom: '', files: [] as string[] };

		for (let index = 0; index < argv.length; index++) {
			const argument = argv[index] ?? '';

			if (argument === '--root') {
				candidate.root = argv[++index] ?? candidate.root;
			} else if (argument === '--files-from') {
				candidate.filesFrom = argv[++index] ?? '';
			} else if (argument !== '') {
				candidate.files.push(argument);
			}
		}

		return new ComplexityCliDto(ComplexityCliDto.#schema.parse(candidate));
	}
}
