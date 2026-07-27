import assert from 'node:assert/strict';
import { test } from 'node:test';
import { SourceDocument } from '#sidecar/syntax/source-document';

test('SourceDocument.of carries its virtual name and text and freezes the value', () => {
	const document = SourceDocument.of('sample.ts', 'const value = 1;\n');

	assert.equal(document.virtualName, 'sample.ts');

	assert.equal(document.text, 'const value = 1;\n');

	assert.equal(Object.isFrozen(document), true);
});

test('SourceDocument.withText keeps the name and swaps the text', () => {
	const document = SourceDocument.of('sample.ts', 'const value = 1;\n');
	const next = document.withText('const value = 2;\n');

	assert.equal(next.virtualName, 'sample.ts');

	assert.equal(next.text, 'const value = 2;\n');

	assert.notEqual(next, document);
});

test('SourceDocument.lineStart returns the offset after the preceding newline', () => {
	const document = SourceDocument.of('sample.ts', 'if (x) {\n\t\tcall();\n}\n');

	assert.equal(document.lineStart(document.text.indexOf('call')), document.text.indexOf('\n') + 1);

	assert.equal(document.lineStart(0), 0);
});

test('SourceDocument.slice reads a range out of the source text', () => {
	const document = SourceDocument.of('sample.ts', 'const value = 1;\n');

	assert.equal(document.slice(6, 11), 'value');
});

test('SourceDocument.lineIndent returns the leading whitespace of the position line', () => {
	const document = SourceDocument.of('sample.ts', 'if (x) {\n\t\tcall();\n}\n');

	assert.equal(document.lineIndent(document.text.indexOf('call')), '\t\t');

	assert.equal(document.lineIndent(0), '');
});

test('SourceDocument.indentUnit reads the unit from the first indented line', () => {
	assert.equal(SourceDocument.of('a.ts', 'function run() {\n    return go();\n}\n').indentUnit(), '    ');

	assert.equal(SourceDocument.of('a.ts', 'function run() {\n\treturn go();\n}\n').indentUnit(), '\t');

	assert.equal(SourceDocument.of('a.ts', 'const value = {\n  key: 1,\n};\n').indentUnit(), '  ');
});

test('SourceDocument.indentUnit falls back to a tab for un-indented source', () => {
	assert.equal(SourceDocument.of('a.ts', 'const value = 1;\n').indentUnit(), '\t');

	assert.equal(SourceDocument.of('a.ts', '').indentUnit(), '\t');
});

test('SourceDocument.indentUnit skips block-comment continuation lines', () => {
	const source = ['/**', ' * Doc comment.', ' */', 'function run() {', '    return go();', '}', ''].join('\n');

	assert.equal(SourceDocument.of('a.ts', source).indentUnit(), '    ');
});

test('SourceDocument.indentUnit reads the unit relative to a baseline-indented block', () => {
	const singleLine = '\t\t\tconst r = builder().withA(1).withB(2).withC(3).build();\n';

	assert.equal(SourceDocument.of('a.ts', singleLine).indentUnit(), '\t');

	const nested = ['\t\t\tfunction run() {', '\t\t\t\treturn go();', '\t\t\t}', ''].join('\n');

	assert.equal(SourceDocument.of('a.ts', nested).indentUnit(), '\t');

	const spacesBaseline = ['      const value = {', '        key: 1,', '      };', ''].join('\n');

	assert.equal(SourceDocument.of('a.ts', spacesBaseline).indentUnit(), '  ');
});
