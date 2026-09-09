import { relative } from 'node:path';
import type { CliCommand } from '#sidecar/cli/command';
import { ComplexityCliDto } from '#sidecar/cli/complexity-cli-dto';
import { ComplexityScanner } from '#sidecar/complexity/complexity-scanner';
import type { ScanFailure, ScoredFunction } from '#sidecar/complexity/complexity-scanner';
import { isErr } from '#sidecar/kernel/result';
import { mapPool } from '#sidecar/kernel/concurrency';
import type { SourceFiles } from '#sidecar/io/source-files';

/** How many files the scan reads and parses at once. */
const SCAN_CONCURRENCY = 8;

/**
 * Scores the requested sources and writes the measurement to stdout as JSON.
 *
 * The command reports numbers only: the limits, the allow list, and the exit
 * policy live in the Go driver, so both language lanes are judged by exactly
 * one implementation of the rules.
 */
export class ComplexityCommand implements CliCommand {
	readonly #sourceFiles: SourceFiles;
	readonly #scanner: ComplexityScanner;

	/**
	 * @param dependencies - The filesystem port and the scorer.
	 * @param dependencies.sourceFiles - Reads the sources to score.
	 * @param dependencies.scanner - Scores one source file.
	 */
	constructor(dependencies: { sourceFiles: SourceFiles; scanner: ComplexityScanner }) {
		this.#sourceFiles = dependencies.sourceFiles;
		this.#scanner = dependencies.scanner;
	}

	/**
	 * Scan the requested files and emit the JSON measurement.
	 *
	 * @param argv - Arguments after the executable and script path.
	 * @returns `0` when every file was scored, `1` when any could not be.
	 */
	async run(argv: readonly string[]): Promise<number> {
		const options = ComplexityCliDto.parse(argv);
		const files = (await this.#targets(options)).filter((file) => {
			return ComplexityCliDto.isScorable(file);
		});

		const functions: ScoredFunction[] = [];
		const errors: ScanFailure[] = [];

		for (const result of await mapPool(files, SCAN_CONCURRENCY, (file) => this.#scanFile(file, options.root))) {
			functions.push(...result.functions);
			errors.push(...result.errors);
		}

		process.stdout.write(`${JSON.stringify({ functions, errors })}\n`);

		return errors.length > 0 ? 1 : 0;
	}

	async #scanFile(file: string, root: string): Promise<{ functions: ScoredFunction[]; errors: ScanFailure[] }> {
		const name = relative(root, file) || file;
		const source = await this.#sourceFiles.readText(file);

		if (isErr(source)) {
			return { functions: [], errors: [{ file: name, message: source.error.message }] };
		}

		return this.#scanner.scan(name, source.value);
	}

	async #targets(options: ComplexityCliDto): Promise<string[]> {
		const files = [...options.files];

		if (options.filesFrom === '') {
			return files;
		}

		const listing = await this.#sourceFiles.readText(options.filesFrom);

		if (isErr(listing)) {
			return files;
		}

		return [...files, ...listing.value.split('\0').filter(Boolean)];
	}
}
