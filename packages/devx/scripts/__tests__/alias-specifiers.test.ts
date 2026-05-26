import assert from 'node:assert/strict';
import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { parseSync } from 'oxc-parser';

type Node = {
	type?: string;
	source?: unknown;
	callee?: unknown;
	arguments?: unknown[];
	value?: unknown;
	[key: string]: unknown;
};

const scriptsDir = fileURLToPath(new URL('..', import.meta.url));

function isRelativeSpecifier(value: string): boolean {
	return value.startsWith('./') || value.startsWith('../');
}

function sourceValue(source: unknown): string | null {
	if (typeof source === 'string') {
		return source;
	}

	if (source && typeof source === 'object' && 'value' in source) {
		const value = (source as { value?: unknown }).value;

		return typeof value === 'string' ? value : null;
	}

	return null;
}

function visit(node: unknown, fn: (node: Node) => void): void {
	if (!node || typeof node !== 'object') {
		return;
	}

	if (Array.isArray(node)) {
		for (const child of node) {
			visit(child, fn);
		}

		return;
	}

	const current = node as Node;

	fn(current);

	for (const value of Object.values(current)) {
		visit(value, fn);
	}
}

async function listTypeScriptFiles(dir: string): Promise<string[]> {
	const entries = await readdir(dir, { recursive: true, withFileTypes: true });
	const files: string[] = [];

	for (const entry of entries) {
		if (!entry.isFile() || !entry.name.endsWith('.ts')) {
			continue;
		}

		files.push(join(entry.parentPath, entry.name));
	}

	return files;
}

function collectModuleSpecifiers(file: string, source: string): string[] {
	const parsed = parseSync(file, source) as unknown as { program: Node };
	const specifiers: string[] = [];

	visit(parsed.program, (node) => {
		if (node.type === 'ImportDeclaration' || node.type === 'ExportNamedDeclaration' || node.type === 'ExportAllDeclaration') {
			const specifier = sourceValue(node.source);

			if (specifier) {
				specifiers.push(specifier);
			}
		}

		if (node.type === 'ImportExpression') {
			const specifier = sourceValue(node.source);

			if (specifier) {
				specifiers.push(specifier);
			}
		}

		if (node.type === 'CallExpression' && (node.callee as Node | undefined)?.type === 'Identifier' && (node.callee as { name?: unknown }).name === 'require') {
			const specifier = sourceValue(node.arguments?.[0]);

			if (specifier) {
				specifiers.push(specifier);
			}
		}
	});

	return specifiers;
}

test('script module specifiers use aliases instead of relative paths', async () => {
	const files = await listTypeScriptFiles(scriptsDir);
	const violations: string[] = [];

	for (const file of files) {
		const source = await readFile(file, 'utf8');
		const specifiers = collectModuleSpecifiers(file, source);

		for (const specifier of specifiers) {
			if (isRelativeSpecifier(specifier)) {
				violations.push(`${file}: ${specifier}`);
			}
		}
	}

	assert.deepEqual(violations, []);
});
