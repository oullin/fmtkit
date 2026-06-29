import { readFile, writeFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { parseSync } from 'oxc-parser';
import { getEnd, getStart, visit } from '#devx/ast';
import { formatDrizzleQueries } from '#devx/drizzle-queries';
import { applyEdits } from '#devx/edits';
import { formatExpandedCalls } from '#devx/expanded-calls';
import type { Edit, Node } from '#devx/types';

const cwd = process.cwd();
const VUE_SCRIPT_REGEX = /(<script\b[^>]*>)([\s\S]*?)(<\/script>)/g;

type ParseResult = {
	program: Node;
	comments?: Node[];
};

type ChainLink = {
	start: number;
	end: number;
	operator: '.' | '?.';
};

type FluentChain = {
	base: Node;
	links: ChainLink[];
};

function isNotFoundError(err: unknown): boolean {
	return typeof err === 'object' && err !== null && 'code' in err && err.code === 'ENOENT';
}

function isTargetFile(path: string): boolean {
	return (path.endsWith('.ts') && !path.endsWith('.d.ts')) || path.endsWith('.vue');
}

function unwrapChainExpression(node: Node | undefined): Node | undefined {
	if (node?.type === 'ChainExpression') {
		return node.expression as Node | undefined;
	}

	return node;
}

function lineIndent(source: string, pos: number): string {
	const lineStart = source.lastIndexOf('\n', pos - 1) + 1;
	const match = source.slice(lineStart, pos).match(/^[ \t]*/);

	return match?.[0] ?? '';
}

function detectIndent(content: string): string {
	const match = content.match(/^[ \t]+(?!\*)(?=\S)/m);

	return match?.[0] ?? '\t';
}

function hasCommentBetween(comments: Node[], from: number, to: number): boolean {
	return comments.some((comment) => {
		const start = getStart(comment);
		const end = getEnd(comment);

		return start >= from && end <= to;
	});
}

function memberCallLink(source: string, member: Node, object: Node, comments: Node[]): ChainLink | null {
	if ((member as { computed?: unknown }).computed) {
		return null;
	}

	const property = member.property as Node | undefined;

	if (!property || (property.type !== 'Identifier' && property.type !== 'PrivateIdentifier')) {
		return null;
	}

	const objectEnd = getEnd(object);
	const propertyStart = getStart(property);

	if (objectEnd < 0 || propertyStart < 0 || propertyStart <= objectEnd) {
		return null;
	}

	if (hasCommentBetween(comments, objectEnd, propertyStart)) {
		return null;
	}

	const separator = source.slice(objectEnd, propertyStart);

	if (separator.includes('//') || separator.includes('/*')) {
		return null;
	}

	const operator = separator.replace(/[ \t\r\n]/g, '');

	if (operator !== '.' && operator !== '?.') {
		return null;
	}

	return {
		start: objectEnd,
		end: propertyStart,
		operator,
	};
}

function collectFluentChain(source: string, outer: Node, comments: Node[]): FluentChain | null {
	let call: Node = outer;

	const links: ChainLink[] = [];

	while (call.type === 'CallExpression') {
		const callee = unwrapChainExpression(call.callee as Node | undefined);

		if (callee?.type !== 'MemberExpression') {
			break;
		}

		const object = unwrapChainExpression(callee.object as Node | undefined);

		if (object?.type !== 'CallExpression') {
			break;
		}

		const link = memberCallLink(source, callee, object, comments);

		if (!link) {
			return null;
		}

		links.push(link);
		call = object;
	}

	if (links.length < 2) {
		return null;
	}

	return {
		base: call,
		links,
	};
}

export function computeFluentChainEdits(content: string, virtualName: string): Edit[] {
	const parsed = parseSync(virtualName, content) as unknown as ParseResult;
	const comments = parsed.comments ?? [];
	const edits = new Map<string, Edit>();
	const indentStep = detectIndent(content);

	visit(parsed.program, (node) => {
		if (node.type !== 'CallExpression') {
			return;
		}

		const chain = collectFluentChain(content, node, comments);

		if (!chain) {
			return;
		}

		const baseStart = getStart(chain.base);

		if (baseStart < 0) {
			return;
		}

		const indent = `${lineIndent(content, baseStart)}${indentStep}`;

		for (const link of chain.links) {
			const replacement = `\n${indent}${link.operator}`;

			if (content.slice(link.start, link.end) === replacement) {
				continue;
			}

			edits.set(`${link.start}:${link.end}`, {
				start: link.start,
				end: link.end,
				replacement,
			});
		}
	});

	return [...edits.values()].sort((a, b) => {
		return a.start - b.start;
	});
}

export function formatFluentChains(content: string, virtualName: string): string {
	const edits = computeFluentChainEdits(content, virtualName);

	const fluentFormatted = edits.length > 0 ? applyEdits(content, edits) : content;
	const drizzleFormatted = formatDrizzleQueries(fluentFormatted, virtualName);

	return formatExpandedCalls(drizzleFormatted, virtualName);
}

function scriptAttribute(openTag: string, name: string): string | null {
	const pattern = new RegExp(`\\b${name}\\s*=\\s*(?:"([^"]*)"|'([^']*)'|([^\\s>]+))`, 'i');
	const match = openTag.match(pattern);

	return match ? (match[1] ?? match[2] ?? match[3]).toLowerCase() : null;
}

function isJavaScriptOrTypeScript(openTag: string): boolean {
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

function processVueFile(original: string, file: string): string {
	let updated = original;

	const segments: { content: string; start: number; virtualName: string }[] = [];

	VUE_SCRIPT_REGEX.lastIndex = 0;

	let match: RegExpExecArray | null;

	while ((match = VUE_SCRIPT_REGEX.exec(original)) !== null) {
		const openTag = match[1];

		if (!isJavaScriptOrTypeScript(openTag)) {
			continue;
		}

		const content = match[2];
		const contentStart = match.index + openTag.length;
		const virtualName = `${file}.script.ts`;

		segments.push({ content, start: contentStart, virtualName });
	}

	for (const segment of [...segments].reverse()) {
		const rewritten = formatFluentChains(segment.content, segment.virtualName);

		if (rewritten === segment.content) {
			continue;
		}

		updated = updated.slice(0, segment.start) + rewritten + updated.slice(segment.start + segment.content.length);
	}

	return updated;
}

async function processFile(file: string, check: boolean): Promise<boolean> {
	const original = await readFile(file, 'utf8');

	const updated = file.endsWith('.vue') ? processVueFile(original, file) : formatFluentChains(original, file);

	if (updated === original) {
		return false;
	}

	if (!check) {
		await writeFile(file, updated);
	}

	return true;
}

async function main(): Promise<void> {
	const rawArgs = process.argv.slice(2);
	const check = rawArgs.includes('--check');

	const files = rawArgs
		.filter((arg) => {
			return arg !== '--check';
		})
		.filter(isTargetFile);

	let changedCount = 0;

	for (const file of files) {
		const changed = await processFile(file, check).catch((err: unknown) => {
			if (isNotFoundError(err)) {
				console.warn(`[fluent-chains] path not found, skipping: ${file}`);

				return false;
			}

			throw err;
		});

		if (!changed) {
			continue;
		}

		changedCount++;
		console.log(`[fluent-chains] ${check ? 'would change' : 'updated'} ${file}`);
	}

	if (check && changedCount > 0) {
		console.error(`[fluent-chains] ${changedCount} file(s) need fluent-chain edits. Run "pnpm format" to fix.`);
		process.exit(1);
	}

	console.log(`[fluent-chains] processed ${files.length} file(s) in ${cwd}, ${changedCount} ${check ? 'would change' : 'changed'}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	main().catch((err: unknown) => {
		console.error(err);
		process.exit(1);
	});
}
