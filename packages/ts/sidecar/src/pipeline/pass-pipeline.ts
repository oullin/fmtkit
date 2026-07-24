import type { EditApplier } from '#sidecar/syntax/edits';
import type { FormattingPass } from '#sidecar/passes/pass';
import type { SourceDocument } from '#sidecar/syntax/source-document';

/** The maximum number of times a step re-runs its pass to reach a fixed point. */
export class IterationBudget {
	/** The upper bound on pass applications within one step. */
	readonly limit: number;

	private constructor(limit: number) {
		this.limit = limit;

		Object.freeze(this);
	}

	/**
	 * Build a budget that runs a pass at most once.
	 *
	 * @returns A single-application budget.
	 */
	static once(): IterationBudget {
		return new IterationBudget(1);
	}

	/**
	 * Build a budget that re-runs a pass until it stabilises or the limit is hit.
	 *
	 * @param limit - The maximum number of applications.
	 * @returns A fixed-point budget bounded by `limit`.
	 */
	static untilStable(limit: number): IterationBudget {
		return new IterationBudget(limit);
	}
}

/** One pipeline stage: a pass driven up to its iteration budget. */
export class PipelineStep {
	/** The formatting pass applied by this step. */
	readonly pass: FormattingPass;

	/** The iteration budget bounding the pass's re-runs. */
	readonly budget: IterationBudget;

	constructor(pass: FormattingPass, budget: IterationBudget) {
		this.pass = pass;
		this.budget = budget;

		Object.freeze(this);
	}

	/**
	 * Apply the pass to a document, re-running it until it stops proposing edits.
	 *
	 * The pass runs at most `budget.limit` times. Each run computes edits against
	 * the current document; an empty result stops the step early and returns the
	 * document unchanged, otherwise the edits are applied and the loop continues.
	 * When the budget is exhausted with a non-empty final run, the applied result
	 * is returned. This reproduces the segment body-wrap fixed point exactly.
	 *
	 * @param document - The document entering the step.
	 * @param edits - The applier used to splice computed edits into the text.
	 * @returns The document after the pass reaches its stable output or budget.
	 */
	apply(document: SourceDocument, edits: EditApplier): SourceDocument {
		let current = document;

		for (let iteration = 0; iteration < this.budget.limit; iteration++) {
			const computed = this.pass.computeEdits(current);

			if (computed.length === 0) {
				return current;
			}

			current = current.withText(edits.apply(current.text, computed));
		}

		return current;
	}
}

/** A named, ordered sequence of pipeline steps applied left-to-right. */
export class PassPipeline {
	/** The reporting label for the pipeline as a whole. */
	readonly name: string;

	readonly #steps: readonly PipelineStep[];
	readonly #edits: EditApplier;

	constructor(name: string, steps: PipelineStep[], edits: EditApplier) {
		this.name = name;
		this.#steps = Object.freeze([...steps]);
		this.#edits = edits;

		Object.freeze(this);
	}

	/**
	 * Fold every step over a document in declaration order.
	 *
	 * Each step always runs, even when an earlier step proposed no edits, so a
	 * no-op step forwards the document unchanged to the next one.
	 *
	 * @param document - The document to format.
	 * @returns The document after every step reaches its stable output.
	 */
	apply(document: SourceDocument): SourceDocument {
		let current = document;

		for (const step of this.#steps) {
			current = step.apply(current, this.#edits);
		}

		return current;
	}
}
