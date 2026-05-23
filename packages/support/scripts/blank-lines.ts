import { readdir, readFile, stat, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { parseSync } from 'oxc-parser';

const cwd = process.cwd();
const check = process.argv.includes('--check');
const candidateDirs = ['src', 'scripts'];

type Node = {
	type: string;
	start?: number;
	end?: number;
	range?: [number, number];
	[key: string]: unknown;
};

type Edit = {
	start: number;
	end: number;
	replacement: string;
};

const STATEMENT_LIST_KEYS: Record<string, 'body' | 'consequent'> = {
	Program: 'body',
	BlockStatement: 'body',
	StaticBlock: 'body',
	SwitchCase: 'consequent',
	ClassBody: 'body',
};

const BLOCK_HAVING_STATEMENTS = new Set(['IfStatement', 'ForStatement', 'ForInStatement', 'ForOfStatement', 'WhileStatement', 'DoWhileStatement', 'SwitchStatement', 'TryStatement']);

const TS_TYPE_DECLARATION_TYPES = new Set(['TSTypeAliasDeclaration', 'TSInterfaceDeclaration']);

const CLASS_METHOD_TYPES = new Set(['MethodDefinition', 'TSAbstractMethodDefinition']);

const CLASS_PROPERTY_TYPES = new Set(['PropertyDefinition', 'TSAbstractPropertyDefinition', 'AccessorProperty', 'TSIndexSignature', 'StaticBlock']);

function getStart(n: Node): number {
	return typeof n.start === 'number' ? n.start : (n.range?.[0] ?? -1);
}

function getEnd(n: Node): number {
	return typeof n.end === 'number' ? n.end : (n.range?.[1] ?? -1);
}

function visit(node: Node, fn: (n: Node) => void): void {
	fn(node);

	for (const key of Object.keys(node)) {
		const value = node[key];

		if (!value || typeof value !== 'object') {
			continue;
		}

		if (Array.isArray(value)) {
			for (const child of value) {
				if (child && typeof child === 'object' && typeof (child as Node).type === 'string') {
					visit(child as Node, fn);
				}
			}
		} else if (typeof (value as Node).type === 'string') {
			visit(value as Node, fn);
		}
	}
}

function collectStatementLists(program: Node): Node[][] {
	const lists: Node[][] = [];

	visit(program, (n) => {
		const key = STATEMENT_LIST_KEYS[n.type];

		if (key) {
			const value = n[key];

			if (Array.isArray(value)) {
				lists.push(value as Node[]);
			}
		}

		if (n.type === 'SwitchStatement' && Array.isArray(n.cases)) {
			lists.push(n.cases as Node[]);
		}
	});

	return lists;
}

function collectClassBodies(program: Node): Node[] {
	const bodies: Node[] = [];

	visit(program, (n) => {
		if (n.type === 'ClassBody') {
			bodies.push(n);
		}
	});

	return bodies;
}

function isExportWithDeclaration(n: Node): boolean {
	if (n.type !== 'ExportNamedDeclaration' && n.type !== 'ExportDefaultDeclaration') {
		return false;
	}

	return Boolean(n.declaration);
}

function needsBlankLineAbove(next: Node): boolean {
	if (next.type === 'ReturnStatement') {
		return true;
	}

	if (next.type === 'SwitchStatement' || next.type === 'SwitchCase' || next.type === 'FunctionDeclaration' || next.type === 'ClassDeclaration') {
		return true;
	}

	return isExportWithDeclaration(next);
}

function isTypeDeclarationBelow(prev: Node): boolean {
	if (TS_TYPE_DECLARATION_TYPES.has(prev.type)) {
		return true;
	}

	if (prev.type === 'ExportNamedDeclaration') {
		const declType = (prev.declaration as Node | undefined)?.type;

		return declType ? TS_TYPE_DECLARATION_TYPES.has(declType) : false;
	}

	return false;
}

function isClassMethodPair(prev: Node, next: Node): boolean {
	return CLASS_METHOD_TYPES.has(prev.type) && CLASS_METHOD_TYPES.has(next.type);
}

function isPropertyToMethodTransition(prev: Node, next: Node): boolean {
	return CLASS_PROPERTY_TYPES.has(prev.type) && CLASS_METHOD_TYPES.has(next.type);
}

function needsBlankLine(prev: Node, next: Node): boolean {
	if (needsBlankLineAbove(next)) {
		return true;
	}

	if (isClassMethodPair(prev, next)) {
		return true;
	}

	if (isPropertyToMethodTransition(prev, next)) {
		return true;
	}

	if (isTypeDeclarationBelow(prev)) {
		return true;
	}

	if (prev.type === 'VariableDeclaration' && next.type !== 'VariableDeclaration') {
		return true;
	}

	if (BLOCK_HAVING_STATEMENTS.has(prev.type)) {
		return true;
	}

	return false;
}

function countNewlines(source: string, from: number, to: number): number {
	let count = 0;

	for (let i = from; i < to; i++) {
		if (source.charCodeAt(i) === 10) {
			count++;
		}
	}

	return count;
}

async function dirExists(dir: string): Promise<boolean> {
	try {
		const s = await stat(dir);

		return s.isDirectory();
	} catch {
		return false;
	}
}

async function listSourceFiles(dir: string): Promise<string[]> {
	const entries = await readdir(dir, { recursive: true, withFileTypes: true });
	const files: string[] = [];

	for (const entry of entries) {
		if (!entry.isFile()) {
			continue;
		}

		if (entry.name.endsWith('.d.ts')) {
			continue;
		}

		if (!entry.name.endsWith('.ts') && !entry.name.endsWith('.vue')) {
			continue;
		}

		files.push(resolve(entry.parentPath, entry.name));
	}

	return files;
}

function classifyMember(node: Node): 'property' | 'constructor' | 'method' {
	if (CLASS_PROPERTY_TYPES.has(node.type)) {
		return 'property';
	}

	if (node.type === 'MethodDefinition' && (node as { kind?: string }).kind === 'constructor') {
		return 'constructor';
	}

	return 'method';
}

function computeClassReorderEdit(source: string, body: Node): Edit | null {
	const members = body.body as Node[] | undefined;

	if (!Array.isArray(members) || members.length < 2) {
		return null;
	}

	const properties: Node[] = [];
	const ctors: Node[] = [];
	const methods: Node[] = [];

	for (const member of members) {
		const kind = classifyMember(member);

		if (kind === 'property') {
			properties.push(member);
		} else if (kind === 'constructor') {
			ctors.push(member);
		} else {
			methods.push(member);
		}
	}

	const desired = [...properties, ...ctors, ...methods];
	const isSameOrder = desired.every((m, i) => m === members[i]);

	if (isSameOrder) {
		return null;
	}

	const bodyStart = getStart(body);
	const bodyEnd = getEnd(body);

	if (bodyStart < 0 || bodyEnd < 0) {
		return null;
	}

	const firstStart = getStart(members[0]);
	const prefix = source.slice(bodyStart + 1, firstStart);
	const indentMatch = prefix.match(/\n([ \t]*)$/);

	if (!indentMatch) {
		return null;
	}

	const indent = indentMatch[1];
	const memberSlices = desired.map((m) => source.slice(getStart(m), getEnd(m)));
	const lastOriginal = members[members.length - 1];
	const closing = source.slice(getEnd(lastOriginal), bodyEnd - 1);
	const replacement = `\n${indent}${memberSlices.join(`\n${indent}`)}${closing}`;

	return {
		start: bodyStart + 1,
		end: bodyEnd - 1,
		replacement,
	};
}

function applyEdits(source: string, edits: Edit[]): string {
	const sorted = [...edits].sort((a, b) => b.start - a.start);
	let out = source;

	for (const edit of sorted) {
		out = out.slice(0, edit.start) + edit.replacement + out.slice(edit.end);
	}

	return out;
}

function computeReorderEdits(content: string, virtualName: string): Edit[] {
	const parsed = parseSync(virtualName, content) as unknown as { program: Node };
	const bodies = collectClassBodies(parsed.program);
	const edits: Edit[] = [];

	for (const body of bodies) {
		const edit = computeClassReorderEdit(content, body);

		if (edit) {
			edits.push(edit);
		}
	}

	return edits;
}

function computeInsertPositions(content: string, virtualName: string, baseOffset: number): number[] {
	const parsed = parseSync(virtualName, content) as unknown as { program: Node };
	const lists = collectStatementLists(parsed.program);
	const positions: number[] = [];

	for (const list of lists) {
		for (let i = 1; i < list.length; i++) {
			const prev = list[i - 1];
			const next = list[i];

			if (!needsBlankLine(prev, next)) {
				continue;
			}

			const prevEnd = getEnd(prev);
			const nextStart = getStart(next);

			if (prevEnd < 0 || nextStart < 0 || nextStart <= prevEnd) {
				continue;
			}

			if (countNewlines(content, prevEnd, nextStart) >= 2) {
				continue;
			}

			const lineStart = content.lastIndexOf('\n', nextStart - 1);

			if (lineStart < 0) {
				continue;
			}

			positions.push(lineStart + 1 + baseOffset);
		}
	}

	return positions;
}

function insertBlankLines(content: string, positions: number[]): string {
	const sorted = [...new Set(positions)].sort((a, b) => b - a);
	let out = content;

	for (const pos of sorted) {
		out = out.slice(0, pos) + '\n' + out.slice(pos);
	}

	return out;
}

function processSegment(content: string, virtualName: string): string {
	const reorderEdits = computeReorderEdits(content, virtualName);
	const reordered = reorderEdits.length > 0 ? applyEdits(content, reorderEdits) : content;
	const positions = computeInsertPositions(reordered, virtualName, 0);

	return insertBlankLines(reordered, positions);
}

const VUE_SCRIPT_REGEX = /(<script\b[^>]*>)([\s\S]*?)(<\/script>)/g;

async function processFile(file: string): Promise<boolean> {
	const original = await readFile(file, 'utf8');
	let updated = original;

	if (file.endsWith('.vue')) {
		const segments: { content: string; start: number; virtualName: string }[] = [];
		VUE_SCRIPT_REGEX.lastIndex = 0;
		let match: RegExpExecArray | null;

		while ((match = VUE_SCRIPT_REGEX.exec(original)) !== null) {
			const openTag = match[1];
			const content = match[2];
			const contentStart = match.index + openTag.length;
			const virtualName = `${file}.script.ts`;

			segments.push({ content, start: contentStart, virtualName });
		}

		for (const segment of [...segments].reverse()) {
			const rewritten = processSegment(segment.content, segment.virtualName);

			if (rewritten === segment.content) {
				continue;
			}

			updated = updated.slice(0, segment.start) + rewritten + updated.slice(segment.start + segment.content.length);
		}
	} else {
		updated = processSegment(original, file);
	}

	if (updated === original) {
		return false;
	}

	if (!check) {
		await writeFile(file, updated);
	}

	return true;
}

const targetDirs: string[] = [];

for (const dir of candidateDirs) {
	const absolute = resolve(cwd, dir);

	if (await dirExists(absolute)) {
		targetDirs.push(absolute);
	}
}

const files = (await Promise.all(targetDirs.map(listSourceFiles))).flat();

let changedCount = 0;

for (const file of files) {
	const changed = await processFile(file);

	if (!changed) {
		continue;
	}

	changedCount++;
	console.log(`[blank-lines] ${check ? 'would change' : 'updated'} ${file}`);
}

if (check && changedCount > 0) {
	console.error(`[blank-lines] ${changedCount} file(s) need blank-line edits. Run "pnpm format" to fix.`);
	process.exit(1);
}

console.log(`[blank-lines] processed ${files.length} file(s) in ${cwd}, ${changedCount} ${check ? 'would change' : 'changed'}`);
