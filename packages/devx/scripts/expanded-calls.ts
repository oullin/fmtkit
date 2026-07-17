import { getEnd, getStart, visit } from '#devx/ast';
import { applyEdits } from '#devx/edits';
import { callParens, hasCommentBetween, isDeclarationFile, lineIndent, nonOverlappingEdits, parseCleanly, sourceOf } from '#devx/pass-utils';
import type { CallParens } from '#devx/pass-utils';
import type { Edit, Node } from '#devx/types';

const FUNCTION_TYPES = new Set(['ArrowFunctionExpression', 'FunctionDeclaration', 'FunctionExpression']);

function unwrapExpression(node: Node | undefined): Node | undefined {
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
		current = current.expression as Node | undefined;
	}

	return current;
}

function calleeParens(source: string, call: Node): CallParens | null {
	return callParens(source, call, unwrapExpression(call.callee as Node | undefined));
}

function callArguments(call: Node): Node[] {
	const args = call.arguments;

	return Array.isArray(args) ? (args as Node[]) : [];
}

function isMethodCall(call: Node): boolean {
	const callee = unwrapExpression(call.callee as Node | undefined);

	return callee?.type === 'MemberExpression';
}

function isComplexArgument(node: Node): boolean {
	const current = unwrapExpression(node);

	return current?.type === 'CallExpression' || current?.type === 'ObjectExpression' || current?.type === 'ArrayExpression';
}

function shouldExpandCall(call: Node): boolean {
	const args = callArguments(call);

	return !isMethodCall(call) && args.length > 0 && args.some(isComplexArgument);
}

function collectParents(node: Node, parents: WeakMap<Node, Node>): void {
	for (const value of Object.values(node)) {
		if (!value || typeof value !== 'object') {
			continue;
		}

		if (Array.isArray(value)) {
			for (const child of value) {
				if (child && typeof child === 'object' && typeof (child as Node).type === 'string') {
					parents.set(child as Node, node);
					collectParents(child as Node, parents);
				}
			}
		} else if (typeof (value as Node).type === 'string') {
			parents.set(value as Node, node);
			collectParents(value as Node, parents);
		}
	}
}

function isInsideCallArgument(node: Node, call: Node): boolean {
	const start = getStart(node);
	const end = getEnd(node);
	const args = callArguments(call);

	return args.some((arg) => {
		return getStart(arg) <= start && end <= getEnd(arg);
	});
}

function nearestCallAncestor(node: Node, parents: WeakMap<Node, Node>): Node | null {
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

function isNestedInsideUnexpandedCallArgument(node: Node, parents: WeakMap<Node, Node>): boolean {
	const ancestor = nearestCallAncestor(node, parents);

	if (!ancestor || !isInsideCallArgument(node, ancestor)) {
		return false;
	}

	return !shouldExpandCall(ancestor);
}

function canUseTrailingComma(arg: Node | undefined): boolean {
	return arg?.type !== 'SpreadElement';
}

/**
 * Re-indent lifted source so its continuation lines match where it now sits.
 *
 * An argument's text is copied out of the call site verbatim, so its second and
 * later lines are still indented relative to the statement the call started on
 * (`from`). Expanding the call moves the argument one or more levels deeper
 * (`to`), and without rebasing those lines they keep the shallower depth and the
 * block reads inside-out. Only the first line is left alone — the caller places
 * it.
 */
function rebaseIndent(text: string, from: string, to: string): string {
	if (from === to || !text.includes('\n')) {
		return text;
	}

	return text
		.split('\n')
		.map((line, index) => {
			if (index === 0) {
				return line;
			}

			if (line.trim() === '') {
				return '';
			}

			return line.startsWith(from) ? `${to}${line.slice(from.length)}` : line;
		})
		.join('\n');
}

function formatCallParens(source: string, call: Node, comments: Node[], indent: string, baseIndent: string): string | null {
	const parens = calleeParens(source, call);
	const args = callArguments(call);

	if (!parens || args.length === 0 || hasCommentBetween(comments, parens.open, parens.close)) {
		return null;
	}

	if (!shouldExpandCall(call)) {
		return null;
	}

	const argIndent = `${indent}\t`;

	const formattedArgs = args.map((arg) => {
		return formatNode(source, arg, comments, argIndent, baseIndent);
	});

	const separator = `,\n${argIndent}`;
	const trailingComma = canUseTrailingComma(args.at(-1)) ? ',' : '';

	return `(\n${argIndent}${formattedArgs.join(separator)}${trailingComma}\n${indent})`;
}

function formatCall(source: string, call: Node, comments: Node[], indent: string, baseIndent: string): string {
	const parens = calleeParens(source, call);
	const formattedParens = formatCallParens(source, call, comments, indent, baseIndent);

	if (!parens || formattedParens === null) {
		return rebaseIndent(sourceOf(source, call), baseIndent, indent);
	}

	return `${source.slice(getStart(call), parens.open)}${formattedParens}`;
}

// indent is where node will sit once expanded; baseIndent is where its source
// text came from and is threaded down unchanged, because nothing has moved in
// the source yet however deep the recursion goes.
function formatNode(source: string, node: Node, comments: Node[], indent: string, baseIndent: string): string {
	if (node.type !== 'CallExpression') {
		return rebaseIndent(sourceOf(source, node), baseIndent, indent);
	}

	if (!shouldExpandCall(node)) {
		return rebaseIndent(sourceOf(source, node), baseIndent, indent);
	}

	return formatCall(source, node, comments, indent, baseIndent);
}

export function computeExpandedCallEdits(content: string, virtualName: string): Edit[] {
	if (isDeclarationFile(virtualName)) {
		return [];
	}

	const parsed = parseCleanly(virtualName, content);

	if (!parsed) {
		return [];
	}

	const comments = parsed.comments;
	const parents = new WeakMap<Node, Node>();
	const edits: Edit[] = [];

	collectParents(parsed.program, parents);

	visit(parsed.program, (node) => {
		if (node.type !== 'CallExpression') {
			return;
		}

		if (!shouldExpandCall(node)) {
			return;
		}

		if (isNestedInsideUnexpandedCallArgument(node, parents)) {
			return;
		}

		const parens = calleeParens(content, node);

		if (!parens || hasCommentBetween(comments, parens.open, parens.close)) {
			return;
		}

		const indent = lineIndent(content, getStart(node));

		// The call has not moved, so every argument's source is still based at
		// this statement's indent — that is what nested levels rebase away from.
		const replacement = formatCallParens(content, node, comments, indent, indent);
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

	return nonOverlappingEdits(edits);
}

export function formatExpandedCalls(content: string, virtualName: string): string {
	const edits = computeExpandedCallEdits(content, virtualName);

	return edits.length > 0 ? applyEdits(content, edits) : content;
}
