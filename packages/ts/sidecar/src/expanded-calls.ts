import { Ast } from '#sidecar/syntax/ast';
import { Edits } from '#sidecar/syntax/edits';
import { FileTargets } from '#sidecar/hosts/file-targets';
import { Node } from '#sidecar/syntax/node-schema';
import { isErr } from '#sidecar/kernel/result';
import { SourceText } from '#sidecar/syntax/source-text';
import type { CallParens } from '#sidecar/syntax/source-text';
import { Sources } from '#sidecar/syntax/sources';
import { TemplateSpans } from '#sidecar/syntax/template-spans';
import type { Edit } from '#sidecar/syntax/edits';

const FUNCTION_TYPES = new Set(['ArrowFunctionExpression', 'FunctionDeclaration', 'FunctionExpression']);

/** Expands structurally complex call arguments into stable multiline layouts. */
export class ExpandedCalls {
	static #unwrapExpression(node: Node | undefined): Node | undefined {
		let current = node;

		while (
			current &&
			(current.type === 'ChainExpression' ||
				current.type === 'ParenthesizedExpression' ||
				current.type === 'TSAsExpression' ||
				current.type === 'TSSatisfiesExpression' ||
				current.type === 'TSNonNullExpression' ||
				current.type === 'TSTypeAssertion')
		) {
			current = Ast.childNode(current, 'expression');
		}

		return current;
	}

	static #calleeParens(source: string, call: Node): CallParens | null {
		return SourceText.callParens(source, call, ExpandedCalls.#unwrapExpression(Ast.childNode(call, 'callee')));
	}

	static #callArguments(call: Node): Node[] {
		return Ast.childNodes(call, 'arguments');
	}

	static #isMethodCall(call: Node): boolean {
		const callee = ExpandedCalls.#unwrapExpression(Ast.childNode(call, 'callee'));

		return callee?.type === 'MemberExpression';
	}

	static #isComplexArgument(node: Node): boolean {
		const current = ExpandedCalls.#unwrapExpression(node);

		return current?.type === 'CallExpression' || current?.type === 'ObjectExpression' || current?.type === 'ArrayExpression';
	}

	static #shouldExpandCall(call: Node): boolean {
		const args = ExpandedCalls.#callArguments(call);

		return !ExpandedCalls.#isMethodCall(call) && args.length > 0 && args.some(ExpandedCalls.#isComplexArgument);
	}

	static #collectParents(node: Node, parents: WeakMap<Node, Node>): void {
		for (const value of Object.values(node)) {
			if (Array.isArray(value)) {
				for (const child of value) {
					if (child instanceof Node) {
						parents.set(child, node);
						ExpandedCalls.#collectParents(child, parents);
					}
				}
			} else if (value instanceof Node) {
				parents.set(value, node);
				ExpandedCalls.#collectParents(value, parents);
			}
		}
	}

	static #isInsideCallArgument(node: Node, call: Node): boolean {
		const start = Ast.getStart(node);
		const end = Ast.getEnd(node);
		const args = ExpandedCalls.#callArguments(call);

		return args.some((arg) => {
			return Ast.getStart(arg) <= start && end <= Ast.getEnd(arg);
		});
	}

	static #nearestCallAncestor(node: Node, parents: WeakMap<Node, Node>): Node | null {
		let current = parents.get(node);

		while (current) {
			if (FUNCTION_TYPES.has(current.type)) {
				return null;
			}

			if (current.type === 'CallExpression') {
				return current;
			}

			current = parents.get(current);
		}

		return null;
	}

	static #isNestedInsideUnexpandedCallArgument(node: Node, parents: WeakMap<Node, Node>): boolean {
		const ancestor = ExpandedCalls.#nearestCallAncestor(node, parents);

		if (!ancestor || !ExpandedCalls.#isInsideCallArgument(node, ancestor)) {
			return false;
		}

		return !ExpandedCalls.#shouldExpandCall(ancestor);
	}

	static #canUseTrailingComma(arg: Node | undefined): boolean {
		return arg?.type !== 'SpreadElement';
	}

	static #rebaseLine(line: string, lineStart: number, from: string, to: string, spans: TemplateSpans): string {
		// A template literal's leading whitespace is string content, not
		// indentation: moving it would rewrite the value, and since oxfmt hugs the
		// expanded call back onto one line before the next run re-expands it, every
		// run would shift the literal one level further right.
		if (spans.contains(lineStart)) {
			return line;
		}

		if (line.trim() === '') {
			return '';
		}

		return line.startsWith(from) ? `${to}${line.slice(from.length)}` : line;
	}

	/**
	 * Re-indent lifted source so its continuation lines match where it now sits.
	 *
	 * A node's text is copied out of the call site verbatim, so its second and
	 * later lines are still indented relative to the line the node was written on.
	 * Expanding the call moves the node one or more levels deeper (`to`), and
	 * without rebasing those lines they keep the shallower depth and the block
	 * reads inside-out. Reading the origin off the node's own line, rather than off
	 * the call being expanded, is what makes a second run a no-op: text already
	 * sitting at its target depth is left alone. Only the first line is skipped
	 * outright — the caller places it.
	 */
	static #rebaseIndent(source: string, node: Node, to: string, spans: TemplateSpans): string {
		const start = Ast.getStart(node);
		const text = SourceText.sourceOf(source, node);
		const from = SourceText.lineIndent(source, start);

		if (from === to || !text.includes('\n')) {
			return text;
		}

		const rebased: string[] = [];

		let lineStart = start;

		for (const [index, line] of text.split('\n').entries()) {
			rebased.push(index === 0 ? line : ExpandedCalls.#rebaseLine(line, lineStart, from, to, spans));
			lineStart += line.length + 1;
		}

		return rebased.join('\n');
	}

	static #formatCallParens(source: string, call: Node, comments: readonly Node[], indent: string, indentUnit: string, spans: TemplateSpans): string | null {
		const parens = ExpandedCalls.#calleeParens(source, call);
		const args = ExpandedCalls.#callArguments(call);

		if (!parens || args.length === 0 || SourceText.hasCommentBetween(comments, parens.open, parens.close)) {
			return null;
		}

		if (!ExpandedCalls.#shouldExpandCall(call)) {
			return null;
		}

		const argIndent = `${indent}${indentUnit}`;

		const formattedArgs = args.map((arg) => {
			return ExpandedCalls.#formatNode(source, arg, comments, argIndent, indentUnit, spans);
		});

		const separator = `,\n${argIndent}`;
		const trailingComma = ExpandedCalls.#canUseTrailingComma(args.at(-1)) ? ',' : '';

		return `(\n${argIndent}${formattedArgs.join(separator)}${trailingComma}\n${indent})`;
	}

	static #formatCall(source: string, call: Node, comments: readonly Node[], indent: string, indentUnit: string, spans: TemplateSpans): string {
		const parens = ExpandedCalls.#calleeParens(source, call);
		const formattedParens = ExpandedCalls.#formatCallParens(source, call, comments, indent, indentUnit, spans);

		if (!parens || formattedParens === null) {
			return ExpandedCalls.#rebaseIndent(source, call, indent, spans);
		}

		return `${source.slice(Ast.getStart(call), parens.open)}${formattedParens}`;
	}

	// indent is where node will sit once expanded; the depth its text came from is
	// read back off the node's own line, because nothing has moved in the source
	// yet however deep the recursion goes.
	static #formatNode(source: string, node: Node, comments: readonly Node[], indent: string, indentUnit: string, spans: TemplateSpans): string {
		if (node.type !== 'CallExpression') {
			return ExpandedCalls.#rebaseIndent(source, node, indent, spans);
		}

		if (!ExpandedCalls.#shouldExpandCall(node)) {
			return ExpandedCalls.#rebaseIndent(source, node, indent, spans);
		}

		return ExpandedCalls.#formatCall(source, node, comments, indent, indentUnit, spans);
	}
	/**
	 * Compute edits for calls whose arguments require a multiline layout.
	 *
	 * @param content - The source text to inspect.
	 * @param virtualName - The filename used to parse the source.
	 * @returns Non-overlapping expanded-call edits.
	 */
	static computeEdits(content: string, virtualName: string): Edit[] {
		if (FileTargets.isDeclarationFile(virtualName)) {
			return [];
		}

		const parsed = Sources.parse(virtualName, content);

		if (isErr(parsed)) {
			return [];
		}

		const comments = parsed.value.comments;
		const parents = new WeakMap<Node, Node>();
		const edits: Edit[] = [];
		const indentUnit = SourceText.detectIndentUnit(content);
		const spans = TemplateSpans.collect(parsed.value.program);

		ExpandedCalls.#collectParents(parsed.value.program, parents);

		Ast.visit(parsed.value.program, (node) => {
			if (node.type !== 'CallExpression') {
				return;
			}

			if (!ExpandedCalls.#shouldExpandCall(node)) {
				return;
			}

			if (ExpandedCalls.#isNestedInsideUnexpandedCallArgument(node, parents)) {
				return;
			}

			const parens = ExpandedCalls.#calleeParens(content, node);

			if (!parens || SourceText.hasCommentBetween(comments, parens.open, parens.close)) {
				return;
			}

			const indent = SourceText.lineIndent(content, Ast.getStart(node));

			const replacement = ExpandedCalls.#formatCallParens(content, node, comments, indent, indentUnit, spans);
			const current = content.slice(parens.open, parens.close + 1);

			if (replacement === null || replacement === current) {
				return;
			}

			edits.push({
				start: parens.open,
				end: parens.close + 1,
				replacement,
			});
		});

		return Edits.nonOverlapping(edits);
	}

	/**
	 * Format calls whose arguments require a multiline layout.
	 *
	 * @param content - The source text to format.
	 * @param virtualName - The filename used to parse the source.
	 * @returns The formatted source, or the original source when no edits apply.
	 */
	static format(content: string, virtualName: string): string {
		const edits = ExpandedCalls.computeEdits(content, virtualName);

		return edits.length > 0 ? Edits.apply(content, edits) : content;
	}
}
