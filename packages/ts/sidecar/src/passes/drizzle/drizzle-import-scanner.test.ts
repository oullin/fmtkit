import assert from 'node:assert/strict';
import { test } from 'node:test';
import { AstReader } from '#sidecar/syntax/ast-reader';
import { DrizzleImportScanner, DrizzleImports } from '#sidecar/passes/drizzle/drizzle-import-scanner';
import { isErr } from '#sidecar/kernel/result';
import { SourceParser } from '#sidecar/syntax/source-parser';

function scan(source: string): DrizzleImports {
	const parsed = new SourceParser().parse('fixture.ts', source);

	assert.equal(isErr(parsed), false);

	if (isErr(parsed)) {
		throw new Error('fixture failed to parse');
	}

	return new DrizzleImportScanner({ ast: new AstReader() }).scan(parsed.value.program);
}

test('DrizzleImportScanner resolves named and aliased Drizzle imports', () => {
	const imports = scan("import { and as all, eq } from 'drizzle-orm';\n");

	assert.equal(imports.isEmpty, false);
	assert.equal(imports.localImport('all'), 'and');
	assert.equal(imports.localImport('eq'), 'eq');
	assert.equal(imports.localImport('missing'), undefined);
	assert.equal(imports.hasNamespace('all'), false);
});

test('DrizzleImportScanner records namespace imports', () => {
	const imports = scan("import * as drizzle from 'drizzle-orm';\n");

	assert.equal(imports.isEmpty, false);
	assert.equal(imports.hasNamespace('drizzle'), true);
	assert.equal(imports.hasNamespace('other'), false);
	assert.equal(imports.localImport('drizzle'), undefined);
});

test('DrizzleImportScanner matches submodule sources', () => {
	const imports = scan("import { sql } from 'drizzle-orm/pg-core';\n");

	assert.equal(imports.localImport('sql'), 'sql');
});

test('DrizzleImportScanner ignores non-Drizzle imports', () => {
	const imports = scan("import { eq } from 'other-orm';\n");

	assert.equal(imports.isEmpty, true);
	assert.equal(imports.localImport('eq'), undefined);
});

test('DrizzleImports.empty carries no bindings and is frozen', () => {
	const imports = DrizzleImports.empty();

	assert.equal(imports.isEmpty, true);
	assert.equal(Object.isFrozen(imports), true);
	assert.equal(imports.localImport('eq'), undefined);
	assert.equal(imports.hasNamespace('drizzle'), false);
});

test('DrizzleImports.of copies its inputs so later mutation is inert', () => {
	const locals = new Map([['eq', 'eq']]);
	const namespaces = new Set(['drizzle']);
	const imports = DrizzleImports.of(locals, namespaces);

	locals.set('and', 'and');
	namespaces.add('other');

	assert.equal(imports.localImport('eq'), 'eq');
	assert.equal(imports.localImport('and'), undefined);
	assert.equal(imports.hasNamespace('drizzle'), true);
	assert.equal(imports.hasNamespace('other'), false);
});
