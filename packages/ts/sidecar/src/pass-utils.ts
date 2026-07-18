import { rename, rm, writeFile } from 'node:fs/promises';
import { parseSync } from 'oxc-parser';
import { childNode, declarationKind, getEnd, getStart, isNode } from '#sidecar/ast';
import type { Edit, Node } from '#sidecar/types';

export type ParseResult = {
	program: Node;
	comments: Node[];
};

export type CallParens = {
	open: number;
	close: number;
};

export type VueScriptBlock = {
	openTag: string;
	content: string;
	start: number;
};

const VUE_SCRIPT_REGEX = /(<script\b[^>]*>)([\s\S]*?)(<\/script>)/g;

let atomicWriteCounter = 0;

// parseCleanly returns null when the source has syntax errors, so passes
// never compute edits from a broken tree.
export function parseCleanly(virtualName: string, content: string): ParseResult | null {
	const parsed = parseSync(virtualName, content);

	if (parsed.errors.length > 0) {
		return null;
	}

	const program: unknown = parsed.program;
	const comments: unknown[] = parsed.comments;

	if (!isNode(program)) {
		return null;
	}

	return { program, comments: comments.filter(isNode) };
}

export function isDeclarationFile(virtualName: string): boolean {
	return virtualName.endsWith('.d.ts');
}

export function isTargetFile(path: string): boolean {
	return (path.endsWith('.ts') && !path.endsWith('.d.ts')) || path.endsWith('.vue');
}

export function isNotFoundError(err: unknown): boolean {
	return typeof err === 'object' && err !== null && 'code' in err && err.code === 'ENOENT';
}

export function sourceOf(source: string, node: Node): string {
	return source.slice(getStart(node), getEnd(node));
}

export function lineStart(source: string, pos: number): number {
	return source.lastIndexOf('\n', pos - 1) + 1;
}

export function lineIndent(source: string, pos: number): string {
	const start = lineStart(source, pos);
	const match = source.slice(start, pos).match(/^[ \t]*/);

	return match?.[0] ?? '';
}

export function hasCommentBetween(comments: Node[], from: number, to: number): boolean {
	return comments.some((comment) => {
		const start = getStart(comment);
		const end = getEnd(comment);

		return start >= from && end <= to;
	});
}

export function unwrapChainExpression(node: Node | undefined): Node | undefined {
	if (node?.type === 'ChainExpression') {
		return childNode(node, 'expression');
	}

	return node;
}

export function isConstDeclaration(node: Node): boolean {
	return node.type === 'VariableDeclaration' && declarationKind(node) === 'const';
}

// callParens locates the argument parentheses of a call whose callee has
// already been unwrapped by the caller's own unwrapping rules.
export function callParens(source: string, call: Node, callee: Node | undefined): CallParens | null {
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

export function rangesOverlap(a: Edit, b: Edit): boolean {
	return a.start < b.end && b.start < a.end;
}

export function nonOverlappingEdits(edits: Edit[]): Edit[] {
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

export function extractVueScripts(content: string): VueScriptBlock[] {
	const blocks: VueScriptBlock[] = [];

	VUE_SCRIPT_REGEX.lastIndex = 0;

	let match: RegExpExecArray | null;

	while ((match = VUE_SCRIPT_REGEX.exec(content)) !== null) {
		const openTag = match[1] ?? '';

		blocks.push({
			openTag,
			content: match[2] ?? '',
			start: match.index + openTag.length,
		});
	}

	return blocks;
}

export function scriptAttribute(openTag: string, name: string): string | null {
	const pattern = new RegExp(`\\b${name}\\s*=\\s*(?:"([^"]*)"|'([^']*)'|([^\\s>]+))`, 'i');
	const match = openTag.match(pattern);
	const value = match ? (match[1] ?? match[2] ?? match[3]) : undefined;

	return value === undefined ? null : value.toLowerCase();
}

export function isJavaScriptOrTypeScript(openTag: string): boolean {
	const lang = scriptAttribute(openTag, 'lang');

	if (lang) {
		return ['ts', 'tsx', 'js', 'jsx', 'typescript', 'javascript'].includes(lang);
	}

	const type = scriptAttribute(openTag, 'type');

	if (type) {
		return type === 'module' || type.includes('javascript') || type.includes('ecmascript');
	}

	return true;
}

// writeFileAtomic writes via a sibling temp file plus rename so a crash
// mid-write can never leave a truncated source file behind.
export async function writeFileAtomic(file: string, content: string): Promise<void> {
	const tmp = `${file}.${process.pid}.${atomicWriteCounter++}.tmp`;

	try {
		await writeFile(tmp, content);

		await rename(tmp, file);
	} catch (err) {
		try {
			await rm(
				tmp,
				{ force: true },
			);
		} catch {
			// Ignore cleanup failures so the original write/rename error surfaces.
		}

		throw err;
	}
}
