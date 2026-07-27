import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { CompositionRoot } from '#sidecar/cli/composition-root';

test('CompositionRoot wires formatAllCommand end-to-end through the Node adapters', async () => {
	const dir = await mkdtemp(
		join(
			tmpdir(),
			'fmtkit-composition-root-',
		),
	);

	try {
		const file = join(dir, 'app.ts');

		await writeFile(file, 'function run() {\n\tconst x = 1;\n\tif (x) return x;\n\treturn 0;\n}\n');

		const exitCode = await CompositionRoot.production()
			.formatAllCommand()
			.run(['--format-files', file, '--syntax-files', file]);

		assert.equal(exitCode, 0);

		const updated = await readFile(file, 'utf8');

		assert.match(updated, /if \(x\) \{\n\t\treturn x;\n\t\}/);
	} finally {
		await rm(
			dir,
			{ recursive: true, force: true },
		);
	}
});

test('CompositionRoot builds every named command', () => {
	const root = CompositionRoot.production();

	assert.equal(typeof root.formatAllCommand().run, 'function');
	assert.equal(typeof root.segmentPassCommand().run, 'function');
	assert.equal(typeof root.fluentPassCommand().run, 'function');
	assert.equal(typeof root.validateSyntaxCommand().run, 'function');
});
