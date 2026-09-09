import { childEntries, childNode, isFunctionNode, stringProperty } from '#sidecar/complexity/ast-entries';
import type { Node } from '#sidecar/syntax/node-schema';

/**
 * The properties each control structure nests. Everything else on the node —
 * a loop's test, a switch's discriminant — stays at the enclosing depth, which
 * is what keeps a condition from being penalised twice.
 */
const NESTED_PROPERTIES = new Map<string, ReadonlySet<string>>([
	['CatchClause', new Set(['body'])],
	['ConditionalExpression', new Set(['alternate', 'consequent'])],
	['DoWhileStatement', new Set(['body'])],
	['ForInStatement', new Set(['body'])],
	['ForOfStatement', new Set(['body'])],
	['ForStatement', new Set(['body'])],
	['SwitchStatement', new Set(['cases'])],
	['WhileStatement', new Set(['body'])],
]);

const JUMP_TYPES: ReadonlySet<string> = new Set(['BreakStatement', 'ContinueStatement']);

/**
 * Scores one function's cognitive complexity under the SonarSource rules.
 *
 * Every control structure costs one plus the depth it sits at; `else` and
 * `else if` cost one without deepening; a run of the same logical operator
 * costs one however long it is; a labelled jump costs one. Nested functions
 * add no increment of their own but do deepen everything inside them, so a
 * closure folds into the named function that declares it.
 */
export class CognitiveScorer {
	/**
	 * Score a function-like node, folding in every function nested below it.
	 *
	 * @param fn - The function, arrow, or method body to score.
	 * @returns The cognitive complexity, zero for a straight-line body.
	 */
	score(fn: Node): number {
		return this.#children(fn, 0);
	}

	#children(node: Node, nesting: number): number {
		let total = 0;

		for (const entry of childEntries(node)) {
			total += this.#node(entry.child, nesting);
		}

		return total;
	}

	#node(node: Node, nesting: number): number {
		if (isFunctionNode(node)) {
			return this.#children(node, nesting + 1);
		}

		if (node.type === 'IfStatement') {
			return this.#branch(node, nesting, 1 + nesting);
		}

		if (node.type === 'LogicalExpression') {
			return this.#logical(node, nesting);
		}

		if (JUMP_TYPES.has(node.type)) {
			return childNode(node, 'label') ? 1 : 0;
		}

		return this.#structure(node, nesting);
	}

	/** Score a control structure and descend, deepening only the properties it nests. */
	#structure(node: Node, nesting: number): number {
		const nested = NESTED_PROPERTIES.get(node.type);
		const increment = nested ? 1 + nesting : 0;

		let total = increment;

		for (const entry of childEntries(node)) {
			total += this.#node(entry.child, nested?.has(entry.key) ? nesting + 1 : nesting);
		}

		return total;
	}

	/**
	 * Score an `if`, its branch, and whatever follows the `else`. An `else if`
	 * re-enters here with an increment of one and the same depth, which is the
	 * whole point of the ladder exemption.
	 */
	#branch(node: Node, nesting: number, increment: number): number {
		let total = increment + this.#optional(node, 'test', nesting) + this.#optional(node, 'consequent', nesting + 1);

		const alternate = childNode(node, 'alternate');

		if (!alternate) {
			return total;
		}

		if (alternate.type === 'IfStatement') {
			return total + this.#branch(alternate, nesting, 1);
		}

		total += 1 + this.#node(alternate, nesting + 1);

		return total;
	}

	#optional(node: Node, key: string, nesting: number): number {
		const child = childNode(node, key);

		return child ? this.#node(child, nesting) : 0;
	}

	/**
	 * Score a logical expression as its runs of one operator. `a && b && c` is
	 * one increment; changing the operator starts a new run.
	 */
	#logical(node: Node, nesting: number): number {
		const operators: string[] = [];
		const operands: Node[] = [];

		this.#flatten(node, operators, operands);

		let runs = 0;
		let previous = '';

		for (const operator of operators) {
			if (operator !== previous) {
				runs += 1;
				previous = operator;
			}
		}

		return operands.reduce((total, operand) => {
			return total + this.#node(operand, nesting);
		}, runs);
	}

	/** Flatten a logical tree into source-order operators and non-logical operands. */
	#flatten(node: Node, operators: string[], operands: Node[]): void {
		this.#side(childNode(node, 'left'), operators, operands);
		operators.push(stringProperty(node, 'operator'));
		this.#side(childNode(node, 'right'), operators, operands);
	}

	#side(node: Node | undefined, operators: string[], operands: Node[]): void {
		if (!node) {
			return;
		}

		if (node.type === 'LogicalExpression') {
			this.#flatten(node, operators, operands);

			return;
		}

		operands.push(node);
	}
}
