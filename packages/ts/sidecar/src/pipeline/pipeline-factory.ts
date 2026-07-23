import { AstReader } from '#sidecar/syntax/ast-reader';
import { BlankLinePass } from '#sidecar/passes/blank-line-pass';
import { BodyWrapPass } from '#sidecar/passes/body-wrap-pass';
import { ClassMemberPolicy } from '#sidecar/passes/policies/class-member-policy';
import { ClassReorderPass } from '#sidecar/passes/class-reorder-pass';
import { DeclarationReorderPass } from '#sidecar/passes/declaration-reorder-pass';
import { EditApplier } from '#sidecar/syntax/edits';
import { IterationBudget, PassPipeline, PipelineStep } from '#sidecar/pipeline/pass-pipeline';
import { SourceParser } from '#sidecar/syntax/source-parser';
import { StatementSpacingPolicy } from '#sidecar/passes/policies/statement-spacing-policy';
import { VueReactivityIdioms } from '#sidecar/passes/policies/vue-reactivity-idioms';

/** The maximum body-wrap iterations before the segment step settles. */
const BODY_WRAP_ITERATIONS = 5;

/** Composes formatting passes into the named pipelines the formatter runs. */
export class PipelineFactory {
	readonly #edits: EditApplier;
	readonly #bodyWrap: BodyWrapPass;
	readonly #classReorder: ClassReorderPass;
	readonly #declarationReorder: DeclarationReorderPass;
	readonly #blankLine: BlankLinePass;

	/**
	 * @param dependencies - The services and policies composed into passes.
	 * @param dependencies.parser - Parses source into a trustworthy tree.
	 * @param dependencies.ast - Traverses and reads validated node fields.
	 * @param dependencies.edits - Splices computed edits into source text.
	 * @param dependencies.members - Classifies class members for reordering.
	 * @param dependencies.spacing - Decides statement blank-line obligations.
	 */
	constructor(dependencies: { parser: SourceParser; ast: AstReader; edits: EditApplier; members: ClassMemberPolicy; spacing: StatementSpacingPolicy }) {
		this.#edits = dependencies.edits;
		this.#bodyWrap = new BodyWrapPass({ parser: dependencies.parser, ast: dependencies.ast });
		this.#classReorder = new ClassReorderPass({ parser: dependencies.parser, ast: dependencies.ast, members: dependencies.members });
		this.#declarationReorder = new DeclarationReorderPass({ parser: dependencies.parser, ast: dependencies.ast });
		this.#blankLine = new BlankLinePass({ parser: dependencies.parser, ast: dependencies.ast, spacing: dependencies.spacing });
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

		return new PipelineFactory({
			parser: new SourceParser(),
			ast,
			edits: new EditApplier(),
			members,
			spacing: new StatementSpacingPolicy({ ast, members, vue }),
		});
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

	// TS-4 adds fluentPipeline() here, composing the fluent-chain, Drizzle-query,
	// and expanded-call passes once those convert to the FormattingPass contract.
}
