import assert from 'node:assert/strict';
import { test } from 'node:test';
import fc from 'fast-check';
import { VueScript } from '#sidecar/hosts/vue-script';
import type { VueScriptBlock } from '#sidecar/hosts/vue-script';

const vueScript = new VueScript();

type GeneratedBlock = {
	readonly content: string;
	readonly javaScriptOrTypeScript: boolean;
	readonly lang: string | null;
	readonly markup: string;
	readonly script: boolean;
};

type AttributeCase = {
	readonly source: string;
	readonly value: string | null;
};

function assertScript(document: string, block: VueScriptBlock | undefined, generated: GeneratedBlock | undefined): void {
	assert.ok(block);
	assert.ok(generated);
	assert.equal(document.slice(block.start, block.start + block.content.length), block.content);
	assert.equal(block.content, generated.content);
	assert.equal(vueScript.attribute(block.openTag, 'lang'), generated.lang);
	assert.equal(vueScript.isJavaScriptOrTypeScript(block.openTag), generated.javaScriptOrTypeScript);
}

const langAttributeArbitrary = fc.constantFrom<AttributeCase>(
	{ source: '', value: null },
	{ source: 'lang="ts"', value: 'ts' },
	{ source: "LANG='TsX'", value: 'tsx' },
	{ source: 'lang=js', value: 'js' },
	{ source: 'lang="javascript"', value: 'javascript' },
	{ source: 'lang="json"', value: 'json' },
);

const typeAttributeArbitrary = fc.constantFrom<AttributeCase>(
	{ source: '', value: null },
	{ source: 'type="module"', value: 'module' },
	{ source: "type='text/javascript'", value: 'text/javascript' },
	{ source: 'type=application/ecmascript', value: 'application/ecmascript' },
	{ source: 'type="application/json"', value: 'application/json' },
);

const scriptContentArbitrary = fc.constantFrom('const value = 1;\n', '\nexport default {};\n', '// embedded script\n');

const scriptBlockArbitrary = fc.tuple(langAttributeArbitrary, typeAttributeArbitrary, scriptContentArbitrary).chain(([lang, type, content]) => {
	const availableAttributes = [lang.source, type.source, 'setup', 'defer', 'data-sidecar="true"'].filter(Boolean);

	return fc.shuffledSubarray(availableAttributes, { maxLength: availableAttributes.length, minLength: availableAttributes.length }).map((attributes) => {
		const openTag = `<script${attributes.length > 0 ? ` ${attributes.join(' ')}` : ''}>`;
		const supportedLangs = ['ts', 'tsx', 'js', 'jsx', 'typescript', 'javascript'];

		const javaScriptOrTypeScript = lang.value
			? supportedLangs.includes(lang.value)
			: type.value
				? type.value === 'module' || type.value.includes('javascript') || type.value.includes('ecmascript')
				: true;

		return {
			content,
			javaScriptOrTypeScript,
			lang: lang.value,
			markup: `${openTag}${content}</script>`,
			script: true,
		};
	});
});

const nonScriptBlockArbitrary = fc
	.tuple(fc.constantFrom('template', 'style'), fc.constantFrom('', ' scoped', ' module', ' data-layout="generated"'), fc.constantFrom('\n<div>fixture</div>\n', '\n.value { color: red; }\n'))
	.map(([tag, attributes, content]): GeneratedBlock => {
		return {
			content,
			javaScriptOrTypeScript: false,
			lang: null,
			markup: `<${tag}${attributes}>${content}</${tag}>`,
			script: false,
		};
	});

const documentArbitrary = fc
	.array(fc.oneof(scriptBlockArbitrary, nonScriptBlockArbitrary), { maxLength: 7, minLength: 1 })
	.filter((blocks) => {
		return blocks.some((block) => block.script);
	})
	.map((blocks) => {
		return {
			document: `${blocks.map((block) => block.markup).join('\n')}\n`,
			scripts: blocks.filter((block) => block.script),
		};
	});

test('vueScript.extractBlocks preserves generated content offsets and language detection', () => {
	fc.assert(
		fc.property(documentArbitrary, ({ document, scripts }) => {
			const extracted = vueScript.extractBlocks(document);

			assert.equal(extracted.length, scripts.length);

			for (let index = 0; index < extracted.length; index++) {
				assertScript(document, extracted[index], scripts[index]);
			}
		}),
		{ numRuns: 100 },
	);
});
