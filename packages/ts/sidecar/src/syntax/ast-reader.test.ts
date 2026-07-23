import assert from 'node:assert/strict';
import { test } from 'node:test';
import { AstReader } from '#sidecar/syntax/ast-reader';
import { Node } from '#sidecar/syntax/node-schema';
import { isErr } from '#sidecar/kernel/result';
import { SourceParser } from '#sidecar/syntax/source-parser';

test('AstReader traverses parsed fixtures and reads validated node fields', () => {
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

	const ast = new AstReader();
	const parsed = new SourceParser().parse('fixture.ts', source);

	assert.equal(isErr(parsed), false);

	if (isErr(parsed)) {
		return;
	}

	const statements = ast.childNodes(parsed.value.program, 'body');
	const importDeclaration = statements[0];
	const variableDeclaration = statements[1];
	const classDeclaration = statements[2];

	assert.equal(ast.childNode(parsed.value.program, 'body'), undefined);

	assert.deepEqual(ast.childNodes(parsed.value.program, 'missing'), []);

	assert.equal(importDeclaration && ast.stringValue(ast.childNode(importDeclaration, 'source') ?? importDeclaration), 'fixture');

	assert.equal(variableDeclaration && ast.declarationKind(variableDeclaration), 'const');

	assert.equal(variableDeclaration && ast.isConstDeclaration(variableDeclaration), true);

	assert.equal(classDeclaration && ast.nodeName(ast.childNode(classDeclaration, 'id') ?? classDeclaration), 'Example');

	assert.equal(classDeclaration && ast.sourceOf(source, classDeclaration).startsWith('class Example'), true);

	const visited: string[] = [];

	ast.visit(parsed.value.program, (node) => {
		visited.push(node.type);
	});

	assert.equal(visited[0], 'Program');

	assert.ok(visited.includes('ReturnStatement'));

	assert.ok(ast.collectStatementLists(parsed.value.program).some((list) => list.some((node) => node.type === 'SwitchCase')));

	assert.equal(ast.collectClassBodies(parsed.value.program).length, 1);

	assert.equal(ast.stringValue(parsed.value.comments[0] ?? parsed.value.program), ' note');
});

test('AstReader position and scalar accessors preserve their fallbacks', () => {
	const ast = new AstReader();
	const ranged = Node.schema.parse({ type: 'Identifier', range: [4, 9], name: 17, kind: false });

	assert.equal(ast.getStart(ranged), 4);

	assert.equal(ast.getEnd(ranged), 9);

	assert.equal(ast.nodeName(ranged), undefined);

	assert.equal(ast.declarationKind(ranged), undefined);

	assert.equal(ast.getStart(Node.schema.parse({ type: 'Identifier' })), -1);
});

test('AstReader.callParens locates argument parentheses and rejects non-calls', () => {
	const ast = new AstReader();
	const source = 'wrap(value);\n';
	const parsed = new SourceParser().parse('fixture.ts', source);

	assert.equal(isErr(parsed), false);

	if (isErr(parsed)) {
		return;
	}

	const statement = ast.childNodes(parsed.value.program, 'body')[0];
	const call = statement && ast.childNode(statement, 'expression');

	assert.ok(call);

	const callee = ast.unwrapChainExpression(ast.childNode(call, 'callee'));
	const parens = ast.callParens(source, call, callee);

	assert.deepEqual(parens, { open: source.indexOf('('), close: source.indexOf(')') });

	assert.equal(ast.callParens(source, call, undefined), null);
});

test('AstReader.unwrapChainExpression returns the wrapped expression or the node itself', () => {
	const ast = new AstReader();
	const parsed = new SourceParser().parse('fixture.ts', 'a?.b();\n');

	assert.equal(isErr(parsed), false);

	if (isErr(parsed)) {
		return;
	}

	const statement = ast.childNodes(parsed.value.program, 'body')[0];
	const expression = statement && ast.childNode(statement, 'expression');

	assert.ok(expression);

	assert.equal(ast.unwrapChainExpression(expression)?.type, 'CallExpression');

	assert.equal(ast.unwrapChainExpression(undefined), undefined);
});
