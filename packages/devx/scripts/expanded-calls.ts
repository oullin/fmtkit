import { parseSync } from 'oxc-parser';
import { getEnd, getStart, visit } from '#devx/ast';
import { applyEdits } from '#devx/edits';
import type { Edit, Node } from '#devx/types';

type ParseResult = {
	program: Node;
	comments?: Node[];
};

type CallParens = {
	open: number;
	close: number;
};

const FUNCTION_TYPES = new Set(['ArrowFunctionExpression', 'FunctionDeclaration', 'FunctionExpression']);

function isDeclarationFile(virtualName: string): boolean {
	return virtualName.endsWith('.d.ts');
}

function sourceOf(source: string, node: Node): string {
	return source.slice(getStart(node), getEnd(node));
}

function lineIndent(source: string, pos: number): string {
	const lineStart = source.lastIndexOf('\n', pos - 1) + 1;
	const match = source.slice(lineStart, pos).match(/^[ \t]*/);

	return match?.[0] ?? '';
}

function hasCommentInside(comments: Node[], from: number, to: number): boolean {
	return comments.some((comment) => {
		const start = getStart(comment);
		const end = getEnd(comment);

		return start >= from && end <= to;
	});
}

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

function callParens(source: string, call: Node): CallParens | null {
	const callee = unwrapExpression(call.callee as Node | undefined);
	const calleeEnd = callee ? getEnd(callee) : -1;
	const callEnd = getEnd(call);

	if (calleeEnd < 0 || callEnd < 0) {
		return null;
	}

	const open = source.indexOf('(', calleeEnd);

	if (open < 0 || open >= callEnd) {
		return null;
	}

	const close = callEnd - 1;

	if (source[close] !== ')') {
		return null;
	}

	return { open, close };
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
	const parens = callParens(source, call);
	const args = callArguments(call);

	if (!parens || args.length === 0 || hasCommentInside(comments, parens.open, parens.close)) {
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
	const parens = callParens(source, call);
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

function rangesOverlap(a: Edit, b: Edit): boolean {
	return a.start < b.end && b.start < a.end;
}

function nonOverlappingEdits(edits: Edit[]): Edit[] {
	const accepted: Edit[] = [];

	const sorted = [...edits].sort((a, b) => {
		return a.start - b.start || b.end - b.start - (a.end - a.start);
	});

	for (const edit of sorted) {
		if (accepted.some((existing) => rangesOverlap(existing, edit))) {
			continue;
		}

		accepted.push(edit);
	}

	return accepted.sort((a, b) => {
		return a.start - b.start;
	});
}

export function computeExpandedCallEdits(content: string, virtualName: string): Edit[] {
	if (isDeclarationFile(virtualName)) {
		return [];
	}

	const parsed = parseSync(virtualName, content) as unknown as ParseResult;
	const comments = parsed.comments ?? [];
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

		const parens = callParens(content, node);

		if (!parens || hasCommentInside(comments, parens.open, parens.close)) {
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
