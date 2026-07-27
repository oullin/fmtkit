import type { CliCommand } from '#sidecar/cli/command';
import type { FormatPipeline } from '#sidecar/pipeline/format-pipeline';
import { SyntaxCliDto } from '#sidecar/cli/syntax-cli-dto';
import type { SyntaxReporter } from '#sidecar/cli/reporter';

/** Runs standalone syntax validation over the CLI's target files. */
export class ValidateSyntaxCommand implements CliCommand {
	readonly #pipeline: FormatPipeline;
	readonly #reporter: SyntaxReporter;

	/**
	 * @param dependencies - The pipeline and reporter used to validate and report.
	 * @param dependencies.pipeline - Validates files and host embedded blocks.
	 * @param dependencies.reporter - Renders parser diagnostics and summary lines.
	 */
	constructor(dependencies: { pipeline: FormatPipeline; reporter: SyntaxReporter }) {
		this.#pipeline = dependencies.pipeline;
		this.#reporter = dependencies.reporter;
	}

	/**
	 * Parse the validation command line, validate, and report the failures.
	 *
	 * @param argv - Arguments after the executable and script path.
	 * @returns `0` when validation succeeds, `1` when it reports a failure.
	 */
	async run(argv: readonly string[]): Promise<number> {
		const options = SyntaxCliDto.parse(argv);
		const files = [...options.files];

		const failures = await this.#pipeline.validate(files);

		return this.#reporter.report(files, failures) ? 0 : 1;
	}
}
