import type { CliCommand } from '#sidecar/cli/command';
import type { FileFormatter } from '#sidecar/pipeline/file-formatter';
import type { FormatPipeline } from '#sidecar/pipeline/format-pipeline';
import { PassCliDto } from '#sidecar/cli/pass-cli-dto';
import type { PassReporter } from '#sidecar/cli/reporter';

/** Runs a single standalone formatting pass over the CLI's target files. */
export class FormatPassCommand implements CliCommand {
	readonly #pipeline: FormatPipeline;
	readonly #formatter: FileFormatter;
	readonly #reporter: PassReporter;
	readonly #label: string;
	readonly #failureNoun: string;

	/**
	 * @param dependencies - The pipeline, formatter, reporter, and labels for the pass.
	 * @param dependencies.pipeline - Applies the formatter across files concurrently.
	 * @param dependencies.formatter - The file formatter whose pipeline drives the pass.
	 * @param dependencies.reporter - Renders per-file and summary reporting lines.
	 * @param dependencies.label - The reporting label the pass emits.
	 * @param dependencies.failureNoun - The change description used in check-mode guidance.
	 */
	constructor(dependencies: { pipeline: FormatPipeline; formatter: FileFormatter; reporter: PassReporter; label: string; failureNoun: string }) {
		this.#pipeline = dependencies.pipeline;
		this.#formatter = dependencies.formatter;
		this.#reporter = dependencies.reporter;
		this.#label = dependencies.label;
		this.#failureNoun = dependencies.failureNoun;
	}

	/**
	 * Parse the pass command line, run the pass, and report its outcomes.
	 *
	 * @param argv - Arguments after the executable and script path.
	 * @returns `0` when the pass succeeds, `1` when it reports a failure.
	 */
	async run(argv: readonly string[]): Promise<number> {
		const options = PassCliDto.parse(argv);
		const files = [...options.files];

		const outcomes = await this.#pipeline.runPass(this.#formatter, files, options.mode);

		return this.#reporter.reportPass(this.#label, files, options.mode, outcomes, this.#failureNoun) ? 0 : 1;
	}
}
