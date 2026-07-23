import { AstReader } from '#sidecar/syntax/ast-reader';
import { BlankLinePass } from '#sidecar/passes/blank-line-pass';
import { BodyWrapPass } from '#sidecar/passes/body-wrap-pass';
import { ClassMemberPolicy } from '#sidecar/passes/policies/class-member-policy';
import { ClassReorderPass } from '#sidecar/passes/class-reorder-pass';
import { DeclarationReorderPass } from '#sidecar/passes/declaration-reorder-pass';
import { DrizzleArgumentWriter } from '#sidecar/passes/drizzle/drizzle-argument-writer';
import { DrizzleCallClassifier } from '#sidecar/passes/drizzle/drizzle-call-classifier';
import { DrizzleImportScanner } from '#sidecar/passes/drizzle/drizzle-import-scanner';
import { DrizzleQueryPass } from '#sidecar/passes/drizzle/drizzle-query-pass';
import { DrizzleVocabulary } from '#sidecar/passes/drizzle/drizzle-vocabulary';
import { EditApplier } from '#sidecar/syntax/edits';
import { EmbeddedBlockSplitter } from '#sidecar/hosts/embedded-block-splitter';
import { ExpandedCallPass } from '#sidecar/passes/expanded-call-pass';
import { FileFormatter } from '#sidecar/pipeline/file-formatter';
import { FileTargetPolicy } from '#sidecar/hosts/file-target-policy';
import { FluentChainPass } from '#sidecar/passes/fluent-chain-pass';
import { IterationBudget, PassPipeline, PipelineStep } from '#sidecar/pipeline/pass-pipeline';
import { MarkdownFences } from '#sidecar/hosts/markdown-fences';
import type { SourceFiles } from '#sidecar/io/source-files';
import { SourceParser } from '#sidecar/syntax/source-parser';
import { StatementSpacingPolicy } from '#sidecar/passes/policies/statement-spacing-policy';
import { SyntaxValidator } from '#sidecar/pipeline/syntax-validator';
import { VueReactivityIdioms } from '#sidecar/passes/policies/vue-reactivity-idioms';
import { VueScript } from '#sidecar/hosts/vue-script';

/** The maximum body-wrap iterations before the segment step settles. */
const BODY_WRAP_ITERATIONS = 5;

/** Composes formatting passes into the named pipelines and formatters the formatter runs. */
export class PipelineFactory {
	readonly #parser: SourceParser;
	readonly #splitter: EmbeddedBlockSplitter;
	readonly #targets: FileTargetPolicy;
	readonly #edits: EditApplier;
	readonly #bodyWrap: BodyWrapPass;
	readonly #classReorder: ClassReorderPass;
	readonly #declarationReorder: DeclarationReorderPass;
	readonly #blankLine: BlankLinePass;
	readonly #fluentChain: FluentChainPass;
	readonly #drizzleQuery: DrizzleQueryPass;
	readonly #expandedCall: ExpandedCallPass;

	/**
	 * @param dependencies - The services and policies composed into passes.
	 * @param dependencies.parser - Parses source into a trustworthy tree.
	 * @param dependencies.ast - Traverses and reads validated node fields.
	 * @param dependencies.edits - Splices computed edits into source text.
	 * @param dependencies.splitter - Extracts and rewrites host embedded blocks.
	 * @param dependencies.targets - Classifies which files each pass and command acts on.
	 * @param dependencies.members - Classifies class members for reordering.
	 * @param dependencies.spacing - Decides statement blank-line obligations.
	 */
	constructor(dependencies: { parser: SourceParser; ast: AstReader; edits: EditApplier; splitter: EmbeddedBlockSplitter; targets: FileTargetPolicy; members: ClassMemberPolicy; spacing: StatementSpacingPolicy }) {
		this.#parser = dependencies.parser;
		this.#splitter = dependencies.splitter;
		this.#targets = dependencies.targets;
		this.#edits = dependencies.edits;
		this.#bodyWrap = new BodyWrapPass({ parser: dependencies.parser, ast: dependencies.ast });
		this.#classReorder = new ClassReorderPass({ parser: dependencies.parser, ast: dependencies.ast, members: dependencies.members });
		this.#declarationReorder = new DeclarationReorderPass({ parser: dependencies.parser, ast: dependencies.ast });
		this.#blankLine = new BlankLinePass({ parser: dependencies.parser, ast: dependencies.ast, spacing: dependencies.spacing });
		this.#fluentChain = new FluentChainPass({ parser: dependencies.parser, ast: dependencies.ast });

		const vocabulary = DrizzleVocabulary.standard();
		const classifier = new DrizzleCallClassifier({ ast: dependencies.ast, vocabulary });

		this.#drizzleQuery = new DrizzleQueryPass({
			parser: dependencies.parser,
			ast: dependencies.ast,
			edits: dependencies.edits,
			scanner: new DrizzleImportScanner({ ast: dependencies.ast }),
			classifier,
			writer: new DrizzleArgumentWriter({ ast: dependencies.ast, vocabulary, classifier }),
			targets: dependencies.targets,
		});

		this.#expandedCall = new ExpandedCallPass({ parser: dependencies.parser, ast: dependencies.ast, edits: dependencies.edits, targets: dependencies.targets });
	}

	/**
	 * Build a factory wired with the default syntax services and policies.
	 *
	 * @returns A factory over freshly constructed, shareable service instances.
	 */
	static create(): PipelineFactory {
		const ast = new AstReader();
		const members = new ClassMemberPolicy({ ast });
		const vue = new VueReactivityIdioms({ ast });
		const splitter = new EmbeddedBlockSplitter({ vueScript: new VueScript(), markdownFences: new MarkdownFences() });

		return new PipelineFactory({
			parser: new SourceParser(),
			ast,
			edits: new EditApplier(),
			splitter,
			targets: new FileTargetPolicy({ embeddedBlocks: splitter }),
			members,
			spacing: new StatementSpacingPolicy({ ast, members, vue }),
		});
	}

	/**
	 * Expose the shared file-target policy the CLI commands filter arguments with.
	 *
	 * @returns The policy shared with the pipeline's declaration-aware passes.
	 */
	fileTargetPolicy(): FileTargetPolicy {
		return this.#targets;
	}

	/**
	 * Build the source-segment pipeline: body wrap, class and declaration
	 * reorder, then blank-line insertion.
	 *
	 * @returns The segment pipeline labelled `blank-lines`.
	 */
	segmentPipeline(): PassPipeline {
		return new PassPipeline(
			'blank-lines',
			[
				new PipelineStep(this.#bodyWrap, IterationBudget.untilStable(BODY_WRAP_ITERATIONS)),
				new PipelineStep(this.#classReorder, IterationBudget.once()),
				new PipelineStep(this.#declarationReorder, IterationBudget.once()),
				new PipelineStep(this.#blankLine, IterationBudget.once()),
			],
			this.#edits,
		);
	}

	/**
	 * Build the fluent pipeline: fluent-chain splitting, then Drizzle-query and
	 * expanded-call formatting over the split source.
	 *
	 * @returns The fluent pipeline labelled `fluent-chains`.
	 */
	fluentPipeline(): PassPipeline {
		return new PassPipeline(
			'fluent-chains',
			[new PipelineStep(this.#fluentChain, IterationBudget.once()), new PipelineStep(this.#drizzleQuery, IterationBudget.once()), new PipelineStep(this.#expandedCall, IterationBudget.once())],
			this.#edits,
		);
	}

	/**
	 * Build a file formatter for the source-segment pipeline.
	 *
	 * @returns A formatter that applies the segment pipeline, host blocks included.
	 */
	segmentFormatter(): FileFormatter {
		return new FileFormatter({ splitter: this.#splitter, pipeline: this.segmentPipeline() });
	}

	/**
	 * Build a file formatter for the fluent pipeline.
	 *
	 * @returns A formatter that applies the fluent pipeline, host blocks included.
	 */
	fluentFormatter(): FileFormatter {
		return new FileFormatter({ splitter: this.#splitter, pipeline: this.fluentPipeline() });
	}

	/**
	 * Build a syntax validator over the factory's splitter and parser.
	 *
	 * @param sourceFiles - The filesystem port the validator reads through.
	 * @returns A validator for TypeScript files and host embedded blocks.
	 */
	syntaxValidator(sourceFiles: SourceFiles): SyntaxValidator {
		return new SyntaxValidator({ sourceFiles, splitter: this.#splitter, parser: this.#parser });
	}
}
