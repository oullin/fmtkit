import assert from 'node:assert/strict';
import { test } from 'node:test';
import fc from 'fast-check';
import { Segment } from '#sidecar/segment';

const identifierArbitrary = fc.constantFrom('alpha', 'beta', 'gamma', 'delta', 'epsilon');
const integerArbitrary = fc.integer({ min: -10, max: 10 });

const singleLineConstArbitrary = fc.tuple(identifierArbitrary, integerArbitrary).map(([name, value]) => {
	return `const ${name} = ${value};`;
});

const multilineConstArbitrary = fc.tuple(identifierArbitrary, integerArbitrary).map(([name, value]) => {
	return [`const ${name} = {`, `\tvalue: ${value},`, '};'].join('\n');
});

const conditionalFunctionArbitrary = fc.tuple(identifierArbitrary, integerArbitrary).map(([name, fallback]) => {
	return [`function read${name}(value: number) {`, '\tif (value > 0) return value;', `\treturn ${fallback};`, '}'].join('\n');
});

const classArbitrary = fc.tuple(identifierArbitrary, integerArbitrary).map(([name, value]) => {
	return [`class ${name}Model {`, '\tread() {', '\t\treturn this.value;', '\t}', `\tvalue = ${value};`, '\tconstructor() {}', '}'].join('\n');
});

const importArbitrary = fc.constantFrom('alpha-package', 'beta-package', 'gamma-package').map((moduleName) => {
	return `import '${moduleName}';`;
});

const templateLiteralArbitrary = identifierArbitrary.map((name) => {
	return 'const ' + name + 'Label = `value-$' + '{1}`;';
});

const commentArbitrary = fc.constantFrom('// formatter note', '/* formatter block note */');
const statementArbitrary = fc.oneof(singleLineConstArbitrary, multilineConstArbitrary, conditionalFunctionArbitrary, classArbitrary, importArbitrary, templateLiteralArbitrary, commentArbitrary);

const sourceArbitrary = fc.array(statementArbitrary, { minLength: 1, maxLength: 8 }).map((statements) => {
	return `${statements.join('\n')}\n`;
});

test('Segment.process is idempotent for composed TypeScript sources', () => {
	fc.assert(
		fc.property(sourceArbitrary, (source) => {
			const once = Segment.process(source, 'property.ts');
			const twice = Segment.process(once, 'property.ts');

			assert.equal(twice, once);
		}),
		{ numRuns: 100 },
	);
});
