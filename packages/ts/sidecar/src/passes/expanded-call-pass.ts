import type { AstReader } from '#sidecar/syntax/ast-reader';
import type { CallParens } from '#sidecar/syntax/ast-reader';
import type { EditApplier } from '#sidecar/syntax/edits';
import { FileTargets } from '#sidecar/hosts/file-targets';
import { Node } from '#sidecar/syntax/node-schema';
import type { ParsedSourceDto } from '#sidecar/syntax/node-schema';
import { isErr } from '#sidecar/kernel/result';
import type { SourceParser } from '#sidecar/syntax/source-parser';
import type { Edit } from '#sidecar/syntax/edits';
import type { FormattingPass } from '#sidecar/passes/pass';
import type { SourceDocument } from '#sidecar/syntax/source-document';
import { TemplateSpans } from '#sidecar/syntax/template-spans';

const FUNCTION_TYPES = new Set(['ArrowFunctionExpression', 'FunctionDeclaration', 'FunctionExpression']);

/** Expands structurally complex call arguments into stable multiline layouts. */
export class ExpandedCallPass implements FormattingPass {
	/** The pass identity used for reporting. */
	readonly name = 'expanded-calls';

	readonly #parser: SourceParser;
	readonly #ast: AstReader;
	readonly #edits: EditApplier;

	/**
	 * @param dependencies - The syntax services consumed by the pass.
	 * @param dependencies.parser - Parses source into a trustworthy tree.
	 * @param dependencies.ast - Traverses and reads validated node fields.
	 * @param dependencies.edits - Reduces candidate edits to a non-overlapping set.
	 */
	constructor(dependencies: { parser: SourceParser; ast: AstReader; edits: EditApplier }) {
		this.#parser = dependencies.parser;
		this.#ast = dependencies.ast;
		this.#edits = dependencies.edits;
	}

	/**
	 * Compute edits for calls whose arguments require a multiline layout.
	 *
	 * @param document - The document to inspect.
	 * @returns Non-overlapping expanded-call edits, or none for invalid source.
	 */
	computeEdits(document: SourceDocument): Edit[] {
		if (FileTargets.isDeclarationFile(document.virtualName)) {
			return [];
		}

		const parsed = this.#parser.parse(document.virtualName, document.text);

		if (isErr(parsed)) {
			return [];
		}

		const parents = new WeakMap<Node, Node>();
		const edits: Edit[] = [];
		const indentUnit = document.indentUnit();
		const spans = TemplateSpans.collect(parsed.value.program);

		this.#collectParents(parsed.value.program, parents);

		this.#ast.visit(parsed.value.program, (node) => {
			if (node.type !== 'CallExpression') {
				return;
			}

			if (!this.#shouldExpandCall(node)) {
				return;
			}

			if (this.#isNestedInsideUnexpandedCallArgument(node, parents)) {
				return;
			}

			const parens = this.#calleeParens(document, node);

			if (!parens || parsed.value.hasCommentBetween(parens.open, parens.close)) {
				return;
			}

			const indent = document.lineIndent(this.#ast.getStart(node));

			const replacement = this.#formatCallParens(document, node, parsed.value, indent, indentUnit, spans);
			const current = document.slice(parens.open, parens.close + 1);

			if (replacement === null || replacement === current) {
				return;
			}

			edits.push({
				start: parens.open,
				end: parens.close + 1,
				replacement,
			});
		});

		return this.#edits.nonOverlapping(edits);
	}

	#unwrapExpression(node: Node | undefined): Node | undefined {
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
			current = this.#ast.childNode(current, 'expression');
		}

		return current;
	}

	#calleeParens(document: SourceDocument, call: Node): CallParens | null {
		return this.#ast.callParens(document.text, call, this.#unwrapExpression(this.#ast.childNode(call, 'callee')));
	}

	#callArguments(call: Node): Node[] {
		return this.#ast.childNodes(call, 'arguments');
	}

	#isMethodCall(call: Node): boolean {
		const callee = this.#unwrapExpression(this.#ast.childNode(call, 'callee'));

		return callee?.type === 'MemberExpression';
	}

	#isComplexArgument(node: Node): boolean {
		const current = this.#unwrapExpression(node);

		return current?.type === 'CallExpression' || current?.type === 'ObjectExpression' || current?.type === 'ArrayExpression';
	}

	#shouldExpandCall(call: Node): boolean {
		const args = this.#callArguments(call);

		return !this.#isMethodCall(call) && args.length > 0 && args.some((argument) => this.#isComplexArgument(argument));
	}

	#collectParents(node: Node, parents: WeakMap<Node, Node>): void {
		for (const value of Object.values(node)) {
			if (Array.isArray(value)) {
				for (const child of value) {
					if (child instanceof Node) {
						parents.set(child, node);
						this.#collectParents(child, parents);
					}
				}
			} else if (value instanceof Node) {
				parents.set(value, node);
				this.#collectParents(value, parents);
			}
		}
	}

	#isInsideCallArgument(node: Node, call: Node): boolean {
		const start = this.#ast.getStart(node);
		const end = this.#ast.getEnd(node);
		const args = this.#callArguments(call);

		return args.some((arg) => {
			return this.#ast.getStart(arg) <= start && end <= this.#ast.getEnd(arg);
		});
	}

	#nearestCallAncestor(node: Node, parents: WeakMap<Node, Node>): Node | null {
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

	#isNestedInsideUnexpandedCallArgument(node: Node, parents: WeakMap<Node, Node>): boolean {
		const ancestor = this.#nearestCallAncestor(node, parents);

		if (!ancestor || !this.#isInsideCallArgument(node, ancestor)) {
			return false;
		}

		return !this.#shouldExpandCall(ancestor);
	}

	#canUseTrailingComma(arg: Node | undefined): boolean {
		return arg?.type !== 'SpreadElement';
	}

	#rebaseLine(line: string, lineStart: number, from: string, to: string, spans: TemplateSpans): string {
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
	#rebaseIndent(document: SourceDocument, node: Node, to: string, spans: TemplateSpans): string {
		const start = this.#ast.getStart(node);
		const text = this.#ast.sourceOf(document.text, node);
		const from = document.lineIndent(start);

		if (from === to || !text.includes('\n')) {
			return text;
		}

		const rebased: string[] = [];

		let lineStart = start;

		for (const [index, line] of text.split('\n').entries()) {
			rebased.push(index === 0 ? line : this.#rebaseLine(line, lineStart, from, to, spans));
			lineStart += line.length + 1;
		}

		return rebased.join('\n');
	}

	#formatCallParens(document: SourceDocument, call: Node, parsed: ParsedSourceDto, indent: string, indentUnit: string, spans: TemplateSpans): string | null {
		const parens = this.#calleeParens(document, call);
		const args = this.#callArguments(call);

		if (!parens || args.length === 0 || parsed.hasCommentBetween(parens.open, parens.close)) {
			return null;
		}

		if (!this.#shouldExpandCall(call)) {
			return null;
		}

		const argIndent = `${indent}${indentUnit}`;

		const formattedArgs = args.map((arg) => {
			return this.#formatNode(document, arg, parsed, argIndent, indentUnit, spans);
		});

		const separator = `,\n${argIndent}`;
		const trailingComma = this.#canUseTrailingComma(args.at(-1)) ? ',' : '';

		return `(\n${argIndent}${formattedArgs.join(separator)}${trailingComma}\n${indent})`;
	}

	#formatCall(document: SourceDocument, call: Node, parsed: ParsedSourceDto, indent: string, indentUnit: string, spans: TemplateSpans): string {
		const parens = this.#calleeParens(document, call);
		const formattedParens = this.#formatCallParens(document, call, parsed, indent, indentUnit, spans);

		if (!parens || formattedParens === null) {
			return this.#rebaseIndent(document, call, indent, spans);
		}

		return `${document.slice(this.#ast.getStart(call), parens.open)}${formattedParens}`;
	}

	// indent is where node will sit once expanded; the depth its text came from is
	// read back off the node's own line, because nothing has moved in the source
	// yet however deep the recursion goes.
	#formatNode(document: SourceDocument, node: Node, parsed: ParsedSourceDto, indent: string, indentUnit: string, spans: TemplateSpans): string {
		if (node.type !== 'CallExpression') {
			return this.#rebaseIndent(document, node, indent, spans);
		}

		if (!this.#shouldExpandCall(node)) {
			return this.#rebaseIndent(document, node, indent, spans);
		}

		return this.#formatCall(document, node, parsed, indent, indentUnit, spans);
	}
}
