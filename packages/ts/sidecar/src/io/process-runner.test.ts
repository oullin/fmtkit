import assert from 'node:assert/strict';
import { chmod, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { NodeProcessRunner } from '#sidecar/io/process-runner';
import { isErr } from '#sidecar/kernel/result';

test('NodeProcessRunner reports successful and failed process exits', async () => {
	const dir = await mkdtemp(
		join(
			tmpdir(),
			'fmtkit-sidecar-runner-',
		),
	);

	try {
		const bin = join(dir, 'fake-oxfmt');

		await writeFile(
			bin,
			`#!/bin/sh
exit "$1"
`,
		);

		await chmod(bin, 0o755);

		const runner = new NodeProcessRunner();

		const succeeded = await runner.run(bin, ['0']);

		const failed = await runner.run(bin, ['7']);

		assert.equal(isErr(succeeded), false);

		assert.equal(isErr(failed) && failed.error.code, 7);

		assert.equal(isErr(failed) && failed.error.signal, null);
	} finally {
		await rm(
			dir,
			{ recursive: true, force: true },
		);
	}
});

test('NodeProcessRunner carries executable spawn failures', async () => {
	const bin = join(
		tmpdir(),
		`fmtkit-sidecar-missing-${process.pid}`,
	);

	const failed = await new NodeProcessRunner().run(bin, []);

	assert.ok(isErr(failed));

	assert.equal(isErr(failed) && failed.error.bin, bin);

	assert.equal(isErr(failed) && failed.error.code, null);

	assert.ok(isErr(failed) && failed.error.cause instanceof Error);
});
