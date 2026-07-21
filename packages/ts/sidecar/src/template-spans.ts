import { Ast } from '#sidecar/ast';
import type { Node } from '#sidecar/types';

type Span = readonly [number, number];

/** Locates the template-literal source that formatting passes must not re-indent. */
export class TemplateSpans {
	readonly #spans: readonly Span[];

	private constructor(spans: Span[]) {
		this.#spans = Object.freeze(spans);

		Object.freeze(this);
	}

	/**
	 * Collect every template literal below a parsed program.
	 *
	 * Whole literals are recorded rather than their quasis, so the interior of a
	 * multiline `${...}` expression is preserved along with the string chunks
	 * around it. Tagged templates need no special case: their `quasi` is itself a
	 * template literal and is visited.
	 *
	 * @param program - The program root to traverse.
	 * @returns The literal spans, in depth-first traversal order.
	 */
	static collect(program: Node): TemplateSpans {
		const spans: Span[] = [];

		Ast.visit(program, (node) => {
			if (node.type !== 'TemplateLiteral') {
				return;
			}

			const start = Ast.getStart(node);
			const end = Ast.getEnd(node);

			if (start >= 0 && end >= 0) {
				spans.push([start, end]);
			}
		});

		return new TemplateSpans(spans);
	}

	/**
	 * Report whether an offset sits inside a template literal's own text.
	 *
	 * The literal's opening offset is excluded because the code that opens the
	 * literal shares that line and may still be re-indented; every later offset,
	 * up to and including the closing backtick, carries string content.
	 *
	 * @param offset - The source offset to test.
	 * @returns `true` when the offset falls inside a literal.
	 */
	contains(offset: number): boolean {
		return this.#spans.some(([start, end]) => {
			return start < offset && offset < end;
		});
	}
}
