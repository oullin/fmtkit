import assert from 'node:assert/strict';
import { test } from 'node:test';
import { ComplexityCommand } from '#sidecar/cli/complexity-command';
import { ComplexityScanner } from '#sidecar/complexity/complexity-scanner';
import { SourceFileUnreadable } from '#sidecar/kernel/errors';
import { err, ok } from '#sidecar/kernel/result';
import type { Result } from '#sidecar/kernel/result';
import type { SourceFileUnwritable } from '#sidecar/kernel/errors';
import type { SourceFiles } from '#sidecar/io/source-files';

/** An in-memory filesystem standing in for the Node source-file port. */
class StubSourceFiles implements SourceFiles {
	readonly #files: ReadonlyMap<string, string>;

	constructor(files: ReadonlyMap<string, string>) {
		this.#files = files;
	}

	/**
	 * Read a staged file.
	 *
	 * @param path - The file to read.
	 * @returns Its staged contents, or an unreadable failure.
	 */
	readText(path: string): Promise<Result<string, SourceFileUnreadable>> {
		const content = this.#files.get(path);

		if (content === undefined) {
			return Promise.resolve(err(new SourceFileUnreadable(path, new Error('missing'))));
		}

		return Promise.resolve(ok(content));
	}

	/**
	 * Reject every write: the complexity scan must never touch source.
	 *
	 * @param path - The file a caller tried to write.
	 * @returns Never; the call fails the test instead.
	 */
	writeTextAtomic(path: string): Promise<Result<void, SourceFileUnwritable>> {
		throw new Error(`the complexity scan wrote ${path}`);
	}
}

/** Run the command with stdout captured, returning its code and output. */
async function run(files: ReadonlyMap<string, string>, argv: string[]): Promise<{ code: number; payload: string }> {
	const original = process.stdout.write.bind(process.stdout);

	let payload = '';

	process.stdout.write = (chunk: string): boolean => {
		payload += chunk;

		return true;
	};

	try {
		const command = new ComplexityCommand({ sourceFiles: new StubSourceFiles(files), scanner: new ComplexityScanner() });

		return { code: await command.run(argv), payload };
	} finally {
		process.stdout.write = original;
	}
}

test('the scan reports every scored function relative to the root', async () => {
	const files = new Map([['/repo/src/app.ts', 'export function read(a: number): number {\n\tif (a > 0) {\n\t\treturn a;\n\t}\n\n\treturn 0;\n}\n']]);

	const { code, payload } = await run(
		files,
		['--root', '/repo', '/repo/src/app.ts'],
	);

	assert.equal(code, 0);
	assert.deepEqual(JSON.parse(payload), {
		functions: [{ key: 'src/app.ts#read', file: 'src/app.ts', line: 1, name: 'read', cyclomatic: 2, cognitive: 1 }],
		errors: [],
	});
});

test('the scan reads its targets from a NUL-separated listing', async () => {
	const files = new Map([
		['/tmp/list', '/repo/src/app.ts\0/repo/src/types.d.ts\0'],
		['/repo/src/app.ts', 'export const read = (): number => 0;\n'],
	]);

	const { code, payload } = await run(
		files,
		['--root', '/repo', '--files-from', '/tmp/list'],
	);

	assert.equal(code, 0);
	assert.equal(JSON.parse(payload).functions.length, 1);
});

test('an unreadable listing leaves the direct targets alone', async () => {
	const files = new Map([['/repo/src/app.ts', 'export const read = (): number => 0;\n']]);

	const { code, payload } = await run(
		files,
		['--root', '/repo', '--files-from', '/tmp/missing', '/repo/src/app.ts'],
	);

	assert.equal(code, 0);
	assert.equal(JSON.parse(payload).functions.length, 1);
});

test('an unreadable source is reported as a file error and fails the run', async () => {
	const { code, payload } = await run(
		new Map(),
		['--root', '/repo', '/repo/src/app.ts'],
	);

	assert.equal(code, 1);
	assert.equal(JSON.parse(payload).errors.length, 1);
});
