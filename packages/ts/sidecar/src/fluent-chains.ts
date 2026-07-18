import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { childNode, getEnd, getStart, visit } from '#sidecar/ast';
import { formatDrizzleQueries } from '#sidecar/drizzle-queries';
import { applyEdits } from '#sidecar/edits';
import { formatExpandedCalls } from '#sidecar/expanded-calls';
import { extractVueScripts, hasCommentBetween, isJavaScriptOrTypeScript, isNotFoundError, isTargetFile, lineIndent, parseCleanly, unwrapChainExpression, writeFileAtomic } from '#sidecar/pass-utils';
import type { Edit, Node } from '#sidecar/types';

const cwd = process.cwd();

type ChainLink = {
	start: number;
	end: number;
	operator: '.' | '?.';
};

type FluentChain = {
	base: Node;
	links: ChainLink[];
};

function detectIndent(content: string): string {
	const match = content.match(/^[ \t]+(?!\*)(?=\S)/m);

	return match?.[0] ?? '\t';
}

function memberCallLink(source: string, member: Node, object: Node, comments: Node[]): ChainLink | null {
	if (member.computed) {
		return null;
	}

	const property = childNode(member, 'property');

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
		const callee = unwrapChainExpression(
			childNode(call, 'callee'),
		);

		if (callee?.type !== 'MemberExpression') {
			break;
		}

		const object = unwrapChainExpression(
			childNode(callee, 'object'),
		);

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
	const parsed = parseCleanly(virtualName, content);

	if (!parsed) {
		return [];
	}

	const comments = parsed.comments;
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

function processVueFile(original: string, file: string): string {
	let updated = original;

	const segments = extractVueScripts(original).filter((segment) => {
		return isJavaScriptOrTypeScript(segment.openTag);
	});

	for (const segment of [...segments].reverse()) {
		const rewritten = formatFluentChains(segment.content, `${file}.script.ts`);

		if (rewritten === segment.content) {
			continue;
		}

		updated = updated.slice(0, segment.start) + rewritten + updated.slice(segment.start + segment.content.length);
	}

	return updated;
}

export async function processFluentChainsFile(file: string, check: boolean): Promise<boolean> {
	const original = await readFile(file, 'utf8');

	const updated = file.endsWith('.vue') ? processVueFile(original, file) : formatFluentChains(original, file);

	if (updated === original) {
		return false;
	}

	if (!check) {
		await writeFileAtomic(file, updated);
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
		const changed = await processFluentChainsFile(file, check).catch((err: unknown) => {
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
