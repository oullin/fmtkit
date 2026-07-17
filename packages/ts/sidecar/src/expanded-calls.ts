import { getEnd, getStart, visit } from '#sidecar/ast';
import { applyEdits } from '#sidecar/edits';
import { callParens, hasCommentBetween, isDeclarationFile, lineIndent, nonOverlappingEdits, parseCleanly, sourceOf } from '#sidecar/pass-utils';
import type { CallParens } from '#sidecar/pass-utils';
import type { Edit, Node } from '#sidecar/types';

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

function formatCallParens(source: string, call: Node, comments: Node[], indent: string): string | null {
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
		return formatNode(source, arg, comments, argIndent);
	});

	const separator = `,\n${argIndent}`;
	const trailingComma = canUseTrailingComma(args.at(-1)) ? ',' : '';

	return `(\n${argIndent}${formattedArgs.join(separator)}${trailingComma}\n${indent})`;
}

function formatCall(source: string, call: Node, comments: Node[], indent: string): string {
	const parens = calleeParens(source, call);
	const formattedParens = formatCallParens(source, call, comments, indent);

	if (!parens || formattedParens === null) {
		return sourceOf(source, call);
	}

	return `${source.slice(getStart(call), parens.open)}${formattedParens}`;
}

function formatNode(source: string, node: Node, comments: Node[], indent: string): string {
	if (node.type !== 'CallExpression') {
		return sourceOf(source, node);
	}

	if (!shouldExpandCall(node)) {
		return sourceOf(source, node);
	}

	return formatCall(source, node, comments, indent);
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

		const replacement = formatCallParens(content, node, comments, indent);
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
