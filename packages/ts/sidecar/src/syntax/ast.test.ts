import assert from 'node:assert/strict';
import { test } from 'node:test';
import { Ast } from '#sidecar/syntax/ast';
import { Node } from '#sidecar/syntax/node-schema';
import { isErr } from '#sidecar/kernel/result';
import { Sources } from '#sidecar/syntax/sources';

test('Ast traverses parsed fixtures and reads validated node fields', () => {
	const source = [
		"import value from 'fixture';",
		'const answer = 42;',
		'class Example {',
		"\tfield = 'ready';",
		'\tmethod() {',
		'\t\treturn this.field;',
		'\t}',
		'}',
		'switch (answer) {',
		'\tcase 42:',
		'\t\tanswer;',
		'}',
		'// note',
		'',
	].join('\n');

	const parsed = Sources.parse('fixture.ts', source);

	assert.equal(isErr(parsed), false);

	if (isErr(parsed)) {
		return;
	}

	const statements = Ast.childNodes(parsed.value.program, 'body');
	const importDeclaration = statements[0];
	const variableDeclaration = statements[1];
	const classDeclaration = statements[2];

	assert.equal(Ast.childNode(parsed.value.program, 'body'), undefined);

	assert.deepEqual(Ast.childNodes(parsed.value.program, 'missing'), []);

	assert.equal(importDeclaration && Ast.stringValue(Ast.childNode(importDeclaration, 'source') ?? importDeclaration), 'fixture');

	assert.equal(variableDeclaration && Ast.declarationKind(variableDeclaration), 'const');

	assert.equal(variableDeclaration && Ast.isConstDeclaration(variableDeclaration), true);

	assert.equal(classDeclaration && Ast.nodeName(Ast.childNode(classDeclaration, 'id') ?? classDeclaration), 'Example');

	assert.equal(classDeclaration && source.slice(Ast.getStart(classDeclaration), Ast.getEnd(classDeclaration)).startsWith('class Example'), true);

	const visited: string[] = [];

	Ast.visit(parsed.value.program, (node) => {
		visited.push(node.type);
	});

	assert.equal(visited[0], 'Program');

	assert.ok(visited.includes('ReturnStatement'));

	assert.ok(Ast.collectStatementLists(parsed.value.program).some((list) => list.some((node) => node.type === 'SwitchCase')));

	assert.equal(Ast.collectClassBodies(parsed.value.program).length, 1);

	assert.equal(Ast.stringValue(parsed.value.comments[0] ?? parsed.value.program), ' note');
});

test('Ast position and scalar accessors preserve their fallbacks', () => {
	const ranged = Node.schema.parse({ type: 'Identifier', range: [4, 9], name: 17, kind: false });

	assert.equal(Ast.getStart(ranged), 4);

	assert.equal(Ast.getEnd(ranged), 9);

	assert.equal(Ast.nodeName(ranged), undefined);

	assert.equal(Ast.declarationKind(ranged), undefined);

	assert.equal(Ast.getStart(Node.schema.parse({ type: 'Identifier' })), -1);
});
