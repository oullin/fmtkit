import assert from 'node:assert/strict';
import { test } from 'node:test';
import { EditApplier } from '#sidecar/syntax/edits';
import { IterationBudget, PassPipeline, PipelineStep } from '#sidecar/pipeline/pass-pipeline';
import { SourceDocument } from '#sidecar/syntax/source-document';
import type { Edit } from '#sidecar/syntax/edits';
import type { FormattingPass } from '#sidecar/passes/pass';

const editApplier = new EditApplier();

/** A pass that appends one marker per run until a run cap is reached. */
class AppendPass implements FormattingPass {
	readonly name: string;

	readonly #marker: string;
	readonly #maxRuns: number;

	#runs = 0;

	constructor(marker: string, maxRuns: number, name = 'append') {
		this.name = name;
		this.#marker = marker;
		this.#maxRuns = maxRuns;
	}

	get runs(): number {
		return this.#runs;
	}

	computeEdits(document: SourceDocument): Edit[] {
		this.#runs++;

		if (this.#runs > this.#maxRuns) {
			return [];
		}

		const end = document.text.length;

		return [{ start: end, end, replacement: this.#marker }];
	}
}

/** A pass that never proposes an edit. */
class NoopPass implements FormattingPass {
	readonly name = 'noop';

	#runs = 0;

	get runs(): number {
		return this.#runs;
	}

	computeEdits(): Edit[] {
		this.#runs++;

		return [];
	}
}

test('IterationBudget.once caps a step at a single application', () => {
	const pass = new AppendPass('x', 10);
	const step = new PipelineStep(pass, IterationBudget.once());

	const result = step.apply(SourceDocument.of('sample.ts', ''), editApplier);

	assert.equal(result.text, 'x');
	assert.equal(pass.runs, 1);
});

test('IterationBudget.untilStable re-runs a pass until it returns no edits', () => {
	const pass = new AppendPass('x', 3);
	const step = new PipelineStep(pass, IterationBudget.untilStable(5));

	const result = step.apply(SourceDocument.of('sample.ts', ''), editApplier);

	assert.equal(result.text, 'xxx');

	// Three productive runs plus the stabilising empty run.
	assert.equal(pass.runs, 4);
});

test('untilStable stops at the limit even when the pass still proposes edits', () => {
	const pass = new AppendPass('x', 10);
	const step = new PipelineStep(pass, IterationBudget.untilStable(5));

	const result = step.apply(SourceDocument.of('sample.ts', ''), editApplier);

	assert.equal(result.text, 'xxxxx');
	assert.equal(pass.runs, 5);
});

test('a no-op pass leaves the document untouched and runs once', () => {
	const pass = new NoopPass();
	const step = new PipelineStep(pass, IterationBudget.untilStable(5));
	const document = SourceDocument.of('sample.ts', 'const a = 1;\n');

	const result = step.apply(document, editApplier);

	assert.equal(result.text, document.text);
	assert.equal(pass.runs, 1);
});

test('an empty pipeline returns its input document unchanged', () => {
	const pipeline = new PassPipeline('empty', [], editApplier);
	const document = SourceDocument.of('sample.ts', 'const a = 1;\n');

	assert.equal(pipeline.apply(document).text, document.text);
});

test('a pipeline folds its steps left-to-right and runs every step', () => {
	const first = new AppendPass('a', 1, 'first');
	const noop = new NoopPass();
	const second = new AppendPass('b', 1, 'second');

	const pipeline = new PassPipeline(
		'compose',
		[new PipelineStep(first, IterationBudget.once()), new PipelineStep(noop, IterationBudget.once()), new PipelineStep(second, IterationBudget.once())],
		editApplier,
	);

	const result = pipeline.apply(SourceDocument.of('sample.ts', ''));

	assert.equal(result.text, 'ab');

	// The middle no-op step still executes between the two productive steps.
	assert.equal(noop.runs, 1);
});

test('the pipeline reports its label and freezes its structure', () => {
	const pipeline = new PassPipeline('blank-lines', [], editApplier);

	assert.equal(pipeline.name, 'blank-lines');
	assert.ok(Object.isFrozen(pipeline));
});
