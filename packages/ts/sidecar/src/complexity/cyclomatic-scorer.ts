import { childEntries, childNode, isFunctionNode, stringProperty } from '#sidecar/complexity/ast-entries';
import type { Node } from '#sidecar/syntax/node-schema';

const DECISION_TYPES: ReadonlySet<string> = new Set([
	'CatchClause',
	'ConditionalExpression',
	'DoWhileStatement',
	'ForInStatement',
	'ForOfStatement',
	'ForStatement',
	'IfStatement',
	'WhileStatement',
]);

const LOGICAL_OPERATORS: ReadonlySet<string> = new Set(['&&', '??', '||']);

const LOGICAL_ASSIGNMENTS: ReadonlySet<string> = new Set(['&&=', '??=', '||=']);

/**
 * Scores one function's cyclomatic complexity under ESLint's `complexity`
 * semantics: one, plus every branch point the function itself owns.
 *
 * Nested functions are scored on their own and add nothing to their parent,
 * which is what makes this number comparable with gocyclo's on the Go side.
 */
export class CyclomaticScorer {
	/**
	 * Score a function-like node.
	 *
	 * @param fn - The function, arrow, or method body to score.
	 * @returns The cyclomatic complexity, never below one.
	 */
	score(fn: Node): number {
		let total = 1;

		for (const entry of childEntries(fn)) {
			total += this.#count(entry.child);
		}

		return total;
	}

	#count(node: Node): number {
		if (isFunctionNode(node)) {
			return 0;
		}

		let total = this.#increment(node);

		for (const entry of childEntries(node)) {
			total += this.#count(entry.child);
		}

		return total;
	}

	#increment(node: Node): number {
		if (node.type === 'SwitchCase') {
			return childNode(node, 'test') ? 1 : 0;
		}

		if (node.type === 'LogicalExpression') {
			return LOGICAL_OPERATORS.has(stringProperty(node, 'operator')) ? 1 : 0;
		}

		if (node.type === 'AssignmentExpression') {
			return LOGICAL_ASSIGNMENTS.has(stringProperty(node, 'operator')) ? 1 : 0;
		}

		return DECISION_TYPES.has(node.type) ? 1 : 0;
	}
}
