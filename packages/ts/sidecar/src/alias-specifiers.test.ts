import assert from 'node:assert/strict';
import { readdir, readFile } from 'node:fs/promises';
import { basename, join } from 'node:path';
import { test } from 'node:test';
import { Ast } from '#sidecar/syntax/ast';
import { isErr } from '#sidecar/kernel/result';
import { Sources } from '#sidecar/syntax/sources';
import type { Node } from '#sidecar/syntax/node-schema';

const sourceExtensions = new Set(['.cjs', '.js', '.jsx', '.mjs', '.ts', '.tsx']);
const exemptFiles = new Set(['sidecar.ts']);

// Root the scan at this test's own directory so it keeps covering every source
// file regardless of how the tree is nested, rather than at a single module's
// resolved location, which would silently shift if that module ever moved.
const scriptsDir = import.meta.dirname;

function isRelativeSpecifier(value: string): boolean {
	return value.startsWith('./') || value.startsWith('../');
}

function sourceValue(source: Node | undefined): string | null {
	return source ? (Ast.stringValue(source) ?? null) : null;
}

function isSourceFile(name: string): boolean {
	for (const extension of sourceExtensions) {
		if (name.endsWith(extension)) {
			return true;
		}
	}

	return false;
}

async function listSourceFiles(dir: string): Promise<string[]> {
	const entries = await readdir(
		dir,
		{ recursive: true, withFileTypes: true },
	);

	const files: string[] = [];

	for (const entry of entries) {
		if (!entry.isFile() || !isSourceFile(entry.name)) {
			continue;
		}

		files.push(join(entry.parentPath, entry.name));
	}

	return files;
}

function collectModuleSpecifiers(file: string, source: string): string[] {
	const parsed = Sources.parse(file, source);
	const specifiers: string[] = [];

	if (isErr(parsed)) {
		return specifiers;
	}

	Ast.visit(parsed.value.program, (node) => {
		if (node.type === 'ImportDeclaration' || node.type === 'ExportNamedDeclaration' || node.type === 'ExportAllDeclaration') {
			const specifier = sourceValue(
				Ast.childNode(node, 'source'),
			);

			if (specifier) {
				specifiers.push(specifier);
			}
		}

		if (node.type === 'ImportExpression') {
			const specifier = sourceValue(
				Ast.childNode(node, 'source'),
			);

			if (specifier) {
				specifiers.push(specifier);
			}
		}

		const callee = Ast.childNode(node, 'callee');

		if (node.type === 'CallExpression' && callee?.type === 'Identifier' && callee.name === 'require') {
			const specifier = sourceValue(Ast.childNodes(node, 'arguments')[0]);

			if (specifier) {
				specifiers.push(specifier);
			}
		}
	});

	return specifiers;
}

test('script module specifiers use aliases instead of relative paths', async () => {
	const files = await listSourceFiles(scriptsDir);

	const violations: string[] = [];

	for (const file of files) {
		if (exemptFiles.has(basename(file))) {
			continue;
		}

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
