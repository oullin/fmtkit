import assert from 'node:assert/strict';
import { test } from 'node:test';
import { isErr } from '#sidecar/result';
import { Sources } from '#sidecar/sources';
import { TemplateSpans } from '#sidecar/template-spans';

function spansOf(source: string): TemplateSpans | null {
	const parsed = Sources.parse('fixture.ts', source);

	return isErr(parsed) ? null : TemplateSpans.collect(parsed.value.program);
}

test('TemplateSpans.contains covers a literal interior but not the line that opens it', () => {
	const source = ['const markup = `', '\t<div>', '\t\t<span>hello</span>', '\t</div>', '`;', ''].join('\n');
	const spans = spansOf(source);

	assert.notEqual(spans, null);

	assert.equal(spans?.contains(source.indexOf('const')), false);

	assert.equal(spans?.contains(source.indexOf('`')), false);

	assert.equal(spans?.contains(source.indexOf('\t<div>')), true);

	assert.equal(spans?.contains(source.indexOf('\t\t<span>')), true);

	// The closing backtick still carries the literal's last line.
	assert.equal(spans?.contains(source.lastIndexOf('`')), true);

	assert.equal(spans?.contains(source.length - 1), false);
});

test('TemplateSpans.collect covers tagged, nested, and interpolated literals', () => {
	const source = ['const query = sql`', '\tselect ${column(`', '\t\tinner', '\t`)}', '`;', ''].join('\n');
	const spans = spansOf(source);

	assert.notEqual(spans, null);

	assert.equal(spans?.contains(source.indexOf('\tselect')), true);

	assert.equal(spans?.contains(source.indexOf('\t\tinner')), true);
});

test('TemplateSpans.contains is false for source without a template literal', () => {
	const spans = spansOf(
		["const value = 'plain';", ''].join('\n'),
	);

	assert.notEqual(spans, null);

	assert.equal(spans?.contains(5), false);
});
