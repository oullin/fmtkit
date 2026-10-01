import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import process from 'node:process';
import { URL } from 'node:url';

const shipped = JSON.parse(readFileSync(new URL('../.oxlintrc.json', import.meta.url), 'utf8'));
const snapshot = JSON.parse(readFileSync(new URL('../tools/oxlint/nkzw-policy.snapshot.json', import.meta.url), 'utf8'));
const upstream = snapshot.config;

assert.equal(snapshot.version, '2.0.1');
assert.match(snapshot.source, /cb48b60893ebf3d6ec0655b295fce70fb51c376a\/index\.js$/);

const normalizeRule = (name) => name.replace(/^@typescript-eslint\//, 'typescript/').replace(/^import-x\//, 'import/');

const existingSettings = {
	eqeqeq: ['error', 'always'],
	'no-unused-vars': ['error', { args: 'after-used', argsIgnorePattern: '^_', caughtErrors: 'none', varsIgnorePattern: '^_' }],
	'unicorn/catch-error-name': ['error', { name: 'cause' }],
};

for (const [name, upstreamValue] of Object.entries(upstream.rules)) {
	const normalized = normalizeRule(name);
	const expected = Object.hasOwn(existingSettings, normalized) ? existingSettings[normalized] : upstreamValue;

	assert.deepEqual(shipped.rules[normalized], expected, `bundled rule ${normalized} drifted from the pinned policy`);
}

assert.equal(shipped.categories.correctness, 'error');
assert.deepEqual(shipped.env, upstream.env);

for (const plugin of upstream.plugins) {
	assert.ok(shipped.plugins.includes(plugin), `missing native plugin ${plugin}`);
}

assert.deepEqual(shipped.jsPlugins, upstream.jsPlugins);
assert.deepEqual(shipped.overrides, [
	{
		...upstream.overrides[0],
		rules: Object.fromEntries(Object.entries(upstream.overrides[0].rules).map(([name, value]) => [normalizeRule(name), value])),
	},
]);

process.stdout.write('bundled Oxlint policy matches the pinned upstream rules and generic override\n');
