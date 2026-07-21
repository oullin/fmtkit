import assert from 'node:assert/strict';
import { test } from 'node:test';
import { SourceText } from '#sidecar/source-text';

test('SourceText.lineIndent returns the leading whitespace of the position line', () => {
	const source = 'if (x) {\n\t\tcall();\n}\n';

	assert.equal(SourceText.lineIndent(source, source.indexOf('call')), '\t\t');

	assert.equal(SourceText.lineIndent(source, 0), '');
});

test('SourceText.detectIndentUnit reads the unit from the first indented line', () => {
	const spaces = 'function run() {\n    return go();\n}\n';

	assert.equal(SourceText.detectIndentUnit(spaces), '    ');

	const tabs = 'function run() {\n\treturn go();\n}\n';

	assert.equal(SourceText.detectIndentUnit(tabs), '\t');

	const twoSpaces = 'const value = {\n  key: 1,\n};\n';

	assert.equal(SourceText.detectIndentUnit(twoSpaces), '  ');
});

test('SourceText.detectIndentUnit falls back to a tab for un-indented source', () => {
	assert.equal(SourceText.detectIndentUnit('const value = 1;\n'), '\t');

	assert.equal(SourceText.detectIndentUnit(''), '\t');
});

test('SourceText.detectIndentUnit skips block-comment continuation lines', () => {
	const source = ['/**', ' * Doc comment.', ' */', 'function run() {', '    return go();', '}', ''].join('\n');

	assert.equal(SourceText.detectIndentUnit(source), '    ');
});
