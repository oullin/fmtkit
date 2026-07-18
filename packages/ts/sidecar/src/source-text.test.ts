import assert from 'node:assert/strict';
import { test } from 'node:test';
import { SourceText } from '#sidecar/source-text';

test('SourceText.lineIndent returns the leading whitespace of the position line', () => {
	const source = 'if (x) {\n\t\tcall();\n}\n';

	assert.equal(SourceText.lineIndent(source, source.indexOf('call')), '\t\t');

	assert.equal(SourceText.lineIndent(source, 0), '');
});
