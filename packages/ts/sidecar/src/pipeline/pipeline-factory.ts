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
	constructor(dependencies: {
		ast: AstReader;
		edits: EditApplier;
		members: ClassMemberPolicy;
		parser: SourceParser;
		spacing: StatementSpacingPolicy;
		splitter: EmbeddedBlockSplitter;
		targets: FileTargetPolicy;
	}) {
		this.#parser = dependencies.parser;
		this.#splitter = dependencies.splitter;
		this.#targets = dependencies.targets;
		this.#edits = dependencies.edits;
		this.#bodyWrap = new BodyWrapPass({ ast: dependencies.ast, parser: dependencies.parser });
		this.#classReorder = new ClassReorderPass({ ast: dependencies.ast, members: dependencies.members, parser: dependencies.parser });
		this.#declarationReorder = new DeclarationReorderPass({ ast: dependencies.ast, parser: dependencies.parser });
		this.#blankLine = new BlankLinePass({ ast: dependencies.ast, parser: dependencies.parser, spacing: dependencies.spacing });
		this.#fluentChain = new FluentChainPass({ ast: dependencies.ast, parser: dependencies.parser });

		const vocabulary = DrizzleVocabulary.standard();
		const classifier = new DrizzleCallClassifier({ ast: dependencies.ast, vocabulary });

		this.#drizzleQuery = new DrizzleQueryPass({
			ast: dependencies.ast,
			classifier,
			edits: dependencies.edits,
			parser: dependencies.parser,
			scanner: new DrizzleImportScanner({ ast: dependencies.ast }),
			targets: dependencies.targets,
			writer: new DrizzleArgumentWriter({ ast: dependencies.ast, classifier, vocabulary }),
		});

		this.#expandedCall = new ExpandedCallPass({ ast: dependencies.ast, edits: dependencies.edits, parser: dependencies.parser, targets: dependencies.targets });
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
		const splitter = new EmbeddedBlockSplitter({ markdownFences: new MarkdownFences(), vueScript: new VueScript() });

		return new PipelineFactory({
			ast,
			edits: new EditApplier(),
			members,
			parser: new SourceParser(),
			spacing: new StatementSpacingPolicy({ ast, members, vue }),
			splitter,
			targets: new FileTargetPolicy({ embeddedBlocks: splitter }),
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
		return new FileFormatter({ pipeline: this.segmentPipeline(), splitter: this.#splitter });
	}

	/**
	 * Build a file formatter for the fluent pipeline.
	 *
	 * @returns A formatter that applies the fluent pipeline, host blocks included.
	 */
	fluentFormatter(): FileFormatter {
		return new FileFormatter({ pipeline: this.fluentPipeline(), splitter: this.#splitter });
	}

	/**
	 * Build a syntax validator over the factory's splitter and parser.
	 *
	 * @param sourceFiles - The filesystem port the validator reads through.
	 * @returns A validator for TypeScript files and host embedded blocks.
	 */
	syntaxValidator(sourceFiles: SourceFiles): SyntaxValidator {
		return new SyntaxValidator({ parser: this.#parser, sourceFiles, splitter: this.#splitter });
	}
}
