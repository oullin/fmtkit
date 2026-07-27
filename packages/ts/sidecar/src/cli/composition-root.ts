import { FormatAllCommand } from '#sidecar/cli/format-all-command';
import { FormatPassCommand } from '#sidecar/cli/format-pass-command';
import { FormatPipeline } from '#sidecar/pipeline/format-pipeline';
import { NodeProcessRunner } from '#sidecar/io/process-runner';
import { NodeSourceFiles } from '#sidecar/io/source-files';
import { PassReporter, SyntaxReporter } from '#sidecar/cli/reporter';
import { PipelineFactory } from '#sidecar/pipeline/pipeline-factory';
import type { ProcessRunner } from '#sidecar/io/process-runner';
import { SourceFileEditor } from '#sidecar/pipeline/source-file-editor';
import type { SourceFiles } from '#sidecar/io/source-files';
import { ValidateSyntaxCommand } from '#sidecar/cli/validate-syntax-command';

/**
 * The single production wiring point for the sidecar CLI. It composes the
 * pass/pipeline graph from {@link PipelineFactory} with the IO adapters, the
 * shared {@link FormatPipeline}, reporters, and the CLI command classes.
 */
export class CompositionRoot {
	readonly #factory: PipelineFactory;
	readonly #pipeline: FormatPipeline;

	/**
	 * @param ports - The Node adapters the pipeline reads and runs through.
	 * @param ports.sourceFiles - The filesystem port for reads and writes.
	 * @param ports.processRunner - The process port for invoking oxfmt.
	 */
	private constructor(ports: { sourceFiles: SourceFiles; processRunner: ProcessRunner }) {
		this.#factory = PipelineFactory.create();
		this.#pipeline = new FormatPipeline({
			editor: new SourceFileEditor({ sourceFiles: ports.sourceFiles }),
			processRunner: ports.processRunner,
			validator: this.#factory.syntaxValidator(ports.sourceFiles),
		});
	}

	/**
	 * Build the production composition root over the Node filesystem and process ports.
	 *
	 * @returns A composition root wired with the default Node adapters.
	 */
	static production(): CompositionRoot {
		return new CompositionRoot({
			sourceFiles: new NodeSourceFiles(),
			processRunner: new NodeProcessRunner(),
		});
	}

	/**
	 * Build the full-pipeline command running the ordered formatting schedule.
	 *
	 * @returns The composed {@link FormatAllCommand}.
	 */
	formatAllCommand(): FormatAllCommand {
		return new FormatAllCommand({
			pipeline: this.#pipeline,
			segmentFormatter: this.#factory.segmentFormatter(),
			fluentFormatter: this.#factory.fluentFormatter(),
			reporter: new PassReporter(),
			syntaxReporter: new SyntaxReporter(),
			targets: this.#factory.fileTargetPolicy(),
		});
	}

	/**
	 * Build the standalone blank-lines segment-pass command.
	 *
	 * @returns The composed segment {@link FormatPassCommand}.
	 */
	segmentPassCommand(): FormatPassCommand {
		return new FormatPassCommand({
			pipeline: this.#pipeline,
			formatter: this.#factory.segmentFormatter(),
			reporter: new PassReporter(),
			targets: this.#factory.fileTargetPolicy(),
			label: 'blank-lines',
			failureNoun: 'blank-line edits',
		});
	}

	/**
	 * Build the standalone fluent-chains pass command.
	 *
	 * @returns The composed fluent {@link FormatPassCommand}.
	 */
	fluentPassCommand(): FormatPassCommand {
		return new FormatPassCommand({
			pipeline: this.#pipeline,
			formatter: this.#factory.fluentFormatter(),
			reporter: new PassReporter(),
			targets: this.#factory.fileTargetPolicy(),
			label: 'fluent-chains',
			failureNoun: 'fluent-chain edits',
		});
	}

	/**
	 * Build the standalone syntax-validation command.
	 *
	 * @returns The composed {@link ValidateSyntaxCommand}.
	 */
	validateSyntaxCommand(): ValidateSyntaxCommand {
		return new ValidateSyntaxCommand({
			pipeline: this.#pipeline,
			reporter: new SyntaxReporter(),
		});
	}
}
