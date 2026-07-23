import { CliOptionsDto } from '#sidecar/cli/format-all-cli-dto';
import type { CliCommand } from '#sidecar/cli/command';
import type { FileFormatter } from '#sidecar/pipeline/file-formatter';
import type { FileTargetPolicy } from '#sidecar/hosts/file-target-policy';
import type { FormatPipeline } from '#sidecar/pipeline/format-pipeline';
import { isErr } from '#sidecar/kernel/result';
import type { PassReporter, SyntaxReporter } from '#sidecar/cli/reporter';

/** Runs the full formatting schedule: segment, oxfmt, fluent, segment, validate. */
export class FormatAllCommand implements CliCommand {
	readonly #pipeline: FormatPipeline;
	readonly #segmentFormatter: FileFormatter;
	readonly #fluentFormatter: FileFormatter;
	readonly #reporter: PassReporter;
	readonly #syntaxReporter: SyntaxReporter;
	readonly #targets: FileTargetPolicy;

	/**
	 * @param dependencies - The pipeline, formatters, and reporters the schedule runs on.
	 * @param dependencies.pipeline - Runs passes, oxfmt, and validation over files.
	 * @param dependencies.segmentFormatter - The blank-lines segment formatter.
	 * @param dependencies.fluentFormatter - The fluent-chains formatter.
	 * @param dependencies.reporter - Renders formatting-pass reporting lines.
	 * @param dependencies.syntaxReporter - Renders syntax-validation reporting lines.
	 * @param dependencies.targets - Classifies the format and syntax target files.
	 */
	constructor(dependencies: { pipeline: FormatPipeline; segmentFormatter: FileFormatter; fluentFormatter: FileFormatter; reporter: PassReporter; syntaxReporter: SyntaxReporter; targets: FileTargetPolicy }) {
		this.#pipeline = dependencies.pipeline;
		this.#segmentFormatter = dependencies.segmentFormatter;
		this.#fluentFormatter = dependencies.fluentFormatter;
		this.#reporter = dependencies.reporter;
		this.#syntaxReporter = dependencies.syntaxReporter;
		this.#targets = dependencies.targets;
	}

	/**
	 * Parse the full-pipeline command line and run the ordered formatting schedule.
	 *
	 * @param argv - Arguments after the executable and script path.
	 * @returns `0` when every stage succeeds, `1` at the first reported failure.
	 */
	async run(argv: readonly string[]): Promise<number> {
		const parsed = CliOptionsDto.parse(argv);

		if (isErr(parsed)) {
			console.error(parsed.error);

			return 1;
		}

		const options = parsed.value;
		const formatTargets = [...new Set(options.formatFiles.filter((file) => this.#targets.isTargetFile(file)))];
		const syntaxTargets = [...new Set(options.syntaxFiles.filter((file) => this.#targets.isSyntaxTarget(file)))];

		const blankLines = await this.#pipeline.runPass(this.#segmentFormatter, formatTargets, options.mode);

		if (!this.#reporter.reportPass('blank-lines', formatTargets, options.mode, blankLines, 'edits')) {
			return 1;
		}

		const oxfmt = await this.#pipeline.runOxfmt({ bin: options.oxfmtBin, config: options.oxfmtConfig, files: formatTargets, mode: options.mode });

		if (isErr(oxfmt)) {
			console.error(oxfmt.error);

			return 1;
		}

		const fluentChains = await this.#pipeline.runPass(this.#fluentFormatter, formatTargets, options.mode);

		if (!this.#reporter.reportPass('fluent-chains', formatTargets, options.mode, fluentChains, 'edits')) {
			return 1;
		}

		// Fluent and expanded calls create blank-line obligations the first pass
		// cannot see, so the second pass makes one invocation reach a fixed point.
		const finalBlankLines = await this.#pipeline.runPass(this.#segmentFormatter, formatTargets, options.mode);

		if (!this.#reporter.reportPass('blank-lines', formatTargets, options.mode, finalBlankLines, 'edits')) {
			return 1;
		}

		if (!this.#syntaxReporter.report(syntaxTargets, await this.#pipeline.validate(syntaxTargets))) {
			return 1;
		}

		return 0;
	}
}
