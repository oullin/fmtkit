import { spawnSync } from 'node:child_process';
import { readFile, stat, writeFile } from 'node:fs/promises';
import { isAbsolute, resolve } from 'node:path';
import { parseSync } from 'oxc-parser';

const cwd = process.cwd();
const rawArgs = process.argv.slice(2);
const check = rawArgs.includes('--check');
const positionalPaths = rawArgs.filter((a) => a !== '--check');

type Node = {
	type: string;
	start?: number;
	end?: number;
	range?: [number, number];
	[key: string]: unknown;
};

const STATEMENT_LIST_KEYS: Record<string, 'body' | 'consequent'> = {
	Program: 'body',
	BlockStatement: 'body',
	StaticBlock: 'body',
	SwitchCase: 'consequent',
};

const BLOCK_HAVING_STATEMENTS = new Set(['IfStatement', 'ForStatement', 'ForInStatement', 'ForOfStatement', 'WhileStatement', 'DoWhileStatement', 'SwitchStatement', 'TryStatement']);
const CONTROL_FLOW_STATEMENTS = new Set(['IfStatement', 'ForStatement', 'ForInStatement', 'ForOfStatement', 'WhileStatement', 'DoWhileStatement', 'SwitchStatement', 'TryStatement']);

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

		if (!key) {
			return;
		}

		const value = n[key];

		if (Array.isArray(value)) {
			lists.push(value as Node[]);
		}
	});

	return lists;
}

function needsBlankLine(prev: Node, next: Node): boolean {
	if (next.type === 'ReturnStatement') {
		return true;
	}

	if (CONTROL_FLOW_STATEMENTS.has(next.type)) {
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

function isTargetFile(p: string): boolean {
	return (p.endsWith('.ts') && !p.endsWith('.d.ts')) || p.endsWith('.vue');
}

function runGitLsFiles(scope?: string): string[] {
	const args = ['ls-files', '--cached', '--others', '--exclude-standard', '-z'];

	if (scope) {
		args.push('--', scope);
	}

	const result = spawnSync('git', args, { cwd, encoding: 'buffer', maxBuffer: 64 * 1024 * 1024 });

	if (result.error || result.status !== 0) {
		const reason = result.error ? result.error.message : `git ls-files exited ${result.status}`;

		console.warn(`[blank-lines] could not list files via git (${reason}); skipping`);

		return [];
	}

	const stdout = result.stdout?.toString('utf8') ?? '';
	const entries = stdout.split('\0').filter((s) => s.length > 0);
	const files: string[] = [];

	for (const entry of entries) {
		if (!isTargetFile(entry)) {
			continue;
		}

		files.push(isAbsolute(entry) ? entry : resolve(cwd, entry));
	}

	return files;
}

async function collectFiles(positional: string[]): Promise<string[]> {
	if (positional.length === 0) {
		return runGitLsFiles();
	}

	const collected: string[] = [];

	for (const raw of positional) {
		const absolute = isAbsolute(raw) ? raw : resolve(cwd, raw);
		const info = await stat(absolute).catch(() => null);

		if (!info) {
			console.warn(`[blank-lines] path not found, skipping: ${absolute}`);

			continue;
		}

		if (info.isFile() || info.isDirectory()) {
			collected.push(...runGitLsFiles(absolute));
		}
	}

	return collected;
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

const VUE_SCRIPT_REGEX = /(<script\b[^>]*>)([\s\S]*?)(<\/script>)/g;

async function processFile(file: string): Promise<boolean> {
	const source = await readFile(file, 'utf8');
	const positions: number[] = [];

	if (file.endsWith('.vue')) {
		VUE_SCRIPT_REGEX.lastIndex = 0;
		let match: RegExpExecArray | null;

		while ((match = VUE_SCRIPT_REGEX.exec(source)) !== null) {
			const openTag = match[1];
			const content = match[2];
			const contentStart = match.index + openTag.length;
			const virtualName = `${file}.script.ts`;

			positions.push(...computeInsertPositions(content, virtualName, contentStart));
		}
	} else {
		positions.push(...computeInsertPositions(source, file, 0));
	}

	if (positions.length === 0) {
		return false;
	}

	const sorted = [...new Set(positions)].sort((a, b) => b - a);
	let out = source;

	for (const pos of sorted) {
		out = out.slice(0, pos) + '\n' + out.slice(pos);
	}

	if (out === source) {
		return false;
	}

	if (!check) {
		await writeFile(file, out);
	}

	return true;
}

async function main(): Promise<void> {
	const files = [...new Set(await collectFiles(positionalPaths))];

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
}

void main();
