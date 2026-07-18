import assert from 'node:assert/strict';
import { join } from 'node:path';
import { test } from 'node:test';
import { OxfmtCliPatcher } from '#oxfmt-inprocess/cli-patcher';
import { ApiExportMissing, CliAlreadyPatched, CliAnchorMissing, CliPatchIncomplete, OxfmtFileUnreadable, OxfmtFileUnwritable, WorkerImportUnrecognised } from '#oxfmt-inprocess/errors';
import { err, isErr, ok } from '#oxfmt-inprocess/result';
import type { Result } from '#oxfmt-inprocess/result';
import { SHIM_MARKER } from '#oxfmt-inprocess/shim-source';
import type { TextFiles } from '#oxfmt-inprocess/text-files';

/** Where the fake oxfmt install lives; only ever a Map key, never touched on disk. */
const DIST_DIR = '/oxfmt/dist';

const CLI_PATH = join(DIST_DIR, 'cli.js');
const WORKER_PATH = join(DIST_DIR, 'cli-worker.js');

/** The anchors the patcher edits by, mirrored from `cli-patcher.ts`. */
const TINYPOOL_IMPORT = 'import Tinypool from "tinypool";';
const REGION_MARKER = '//#region src-js/cli/worker-proxy.ts';
const RUNTIME_ANCHOR = 'runtime: "child_process"';

/**
 * An in-memory `TextFiles`, so the patcher's behaviour is observed through
 * the results it returns and the contents it stores — no filesystem, no spies.
 *
 * A missing key reads as `OxfmtFileUnreadable`; a path listed as unwritable
 * writes as `OxfmtFileUnwritable`.
 */
class FakeTextFiles implements TextFiles {
	/** The stored files, exposed so tests can assert on written contents. */
	readonly files: Map<string, string>;

	readonly #unwritable: ReadonlySet<string>;

	/**
	 * @param files - The initial path-to-contents entries.
	 * @param unwritable - Paths whose writes fail, for error-branch tests.
	 */
	constructor(files: Iterable<readonly [string, string]>, unwritable: Iterable<string> = []) {
		this.files = new Map(files);
		this.#unwritable = new Set(unwritable);
	}

	/**
	 * Read a stored file.
	 *
	 * @param path - The file to read.
	 * @returns The stored contents, or `OxfmtFileUnreadable` for a missing key.
	 */
	readText(path: string): Result<string, OxfmtFileUnreadable> {
		const contents = this.files.get(path);

		if (contents === undefined) {
			return err(new OxfmtFileUnreadable(path, new Error('no such file in fake')));
		}

		return ok(contents);
	}

	/**
	 * Store a file, unless its path was marked unwritable.
	 *
	 * @param path - The file to write.
	 * @param contents - The contents to store.
	 * @returns Nothing, or `OxfmtFileUnwritable` for a poisoned path.
	 */
	writeText(path: string, contents: string): Result<void, OxfmtFileUnwritable> {
		if (this.#unwritable.has(path)) {
			return err(new OxfmtFileUnwritable(path, new Error('write refused by fake')));
		}

		this.files.set(path, contents);

		return ok(undefined);
	}
}

/**
 * A minimal `cli.js` shaped like oxfmt's real bundle: the Tinypool import, a
 * `worker-proxy` region holding every worker-pool reference (`Tinypool`,
 * `pool.run`, `pool = `, the child_process runtime), and `runCli` wiring
 * outside the region that the rewrite must leave untouched.
 */
const CLI_SOURCE = `import { runCli, toFormatFileResult, toNullable } from "./chunks/cli-main.js";
${TINYPOOL_IMPORT}
${REGION_MARKER}
let pool;
async function initExternalFormatter(numThreads) {
	pool = new Tinypool({
		filename: new URL("./cli-worker.js", import.meta.url).href,
		maxThreads: numThreads,
		${RUNTIME_ANCHOR},
	});
}
async function disposeExternalFormatter() {
	await pool.destroy();
}
function formatFile(options, code) {
	return toFormatFileResult(pool.run({ options, code }, { name: "formatFile" }));
}
function formatEmbeddedCode(options, code) {
	return toNullable(pool.run({ options, code }, { name: "formatEmbeddedCode" }));
}
//#endregion
runCli({ formatFile, formatEmbeddedCode, initExternalFormatter, disposeExternalFormatter });
`;

/**
 * A minimal `cli-worker.js` shaped like oxfmt's real worker entry: the four
 * API functions re-exported under rolldown's single-letter aliases from a
 * content-hashed module.
 */
const WORKER_SOURCE = `import { i as sortTailwindClasses, n as formatEmbeddedDoc, r as formatFile, t as formatEmbeddedCode } from './apis-CvFX8LhR.js';
export { formatEmbeddedCode, formatEmbeddedDoc, formatFile, sortTailwindClasses };
`;

/** A fake holding a pristine oxfmt install, ready to patch. */
function pristineInstall(): FakeTextFiles {
	return new FakeTextFiles([
		[CLI_PATH, CLI_SOURCE],
		[WORKER_PATH, WORKER_SOURCE],
	]);
}

/**
 * Narrow a result to its success branch.
 *
 * @param result - The result to unwrap.
 * @returns The success value; fails the test on an error result.
 */
function unwrapOk<T, E extends Error>(result: Result<T, E>): T {
	if (isErr(result)) {
		assert.fail(`expected ok, got ${result.error.name}: ${result.error.message}`);
	}

	return result.value;
}

/**
 * Narrow a result to its failure branch.
 *
 * @param result - The result to unwrap.
 * @returns The error; fails the test on a success result.
 */
function unwrapErr<T, E extends Error>(result: Result<T, E>): E {
	if (!isErr(result)) {
		assert.fail('expected err, got ok');
	}

	return result.error;
}

test('patch() rewrites the CLI in place and reports what changed', () => {
	const files = pristineInstall();

	const outcome = unwrapOk(
		new OxfmtCliPatcher(files).patch(DIST_DIR),
	);

	assert.equal(outcome.cliPath, CLI_PATH);
	assert.equal(outcome.apiModuleSpecifier, './apis-CvFX8LhR.js');

	const patched = files.files.get(CLI_PATH);

	assert.ok(patched !== undefined, 'the rewritten CLI was stored');

	// The shim is in, marked so a rerun is detected.
	assert.ok(patched.includes(SHIM_MARKER));
	assert.ok(patched.includes('import { r as __fmtkitFormatFile, t as __fmtkitFormatEmbeddedCode, n as __fmtkitFormatEmbeddedDoc, i as __fmtkitSortTailwindClasses } from "./apis-CvFX8LhR.js";'));

	// The worker pool is gone, wholesale.
	assert.ok(!patched.includes('Tinypool'));
	assert.ok(!patched.includes('pool.run'));
	assert.ok(!patched.includes('pool = '));

	// The wiring outside the region is untouched, and the worker entry unwritten.
	assert.ok(patched.includes('runCli({ formatFile, formatEmbeddedCode, initExternalFormatter, disposeExternalFormatter });'));
	assert.equal(files.files.get(WORKER_PATH), WORKER_SOURCE);
});

test('patch() on an already-patched CLI reports CliAlreadyPatched instead of nesting', () => {
	const files = pristineInstall();
	const patcher = new OxfmtCliPatcher(files);

	unwrapOk(
		patcher.patch(DIST_DIR),
	);

	const error = unwrapErr(
		patcher.patch(DIST_DIR),
	);

	assert.ok(error instanceof CliAlreadyPatched);
	assert.equal(error._tag, 'CliAlreadyPatched');
	assert.equal(error.path, CLI_PATH);
});

test('patch() reports OxfmtFileUnreadable when cli.js cannot be read', () => {
	const files = new FakeTextFiles([[WORKER_PATH, WORKER_SOURCE]]);

	const error = unwrapErr(
		new OxfmtCliPatcher(files).patch(DIST_DIR),
	);

	assert.ok(error instanceof OxfmtFileUnreadable);
	assert.equal(error._tag, 'OxfmtFileUnreadable');
	assert.equal(error.path, CLI_PATH);
});

test('patch() reports OxfmtFileUnreadable when cli-worker.js cannot be read', () => {
	const files = new FakeTextFiles([[CLI_PATH, CLI_SOURCE]]);

	const error = unwrapErr(
		new OxfmtCliPatcher(files).patch(DIST_DIR),
	);

	assert.ok(error instanceof OxfmtFileUnreadable);
	assert.equal(error._tag, 'OxfmtFileUnreadable');
	assert.equal(error.path, WORKER_PATH);
});

test('patch() reports WorkerImportUnrecognised when the worker entry has no API import', () => {
	const files = new FakeTextFiles([
		[CLI_PATH, CLI_SOURCE],
		[WORKER_PATH, 'export {};\n'],
	]);

	const error = unwrapErr(
		new OxfmtCliPatcher(files).patch(DIST_DIR),
	);

	assert.ok(error instanceof WorkerImportUnrecognised);
	assert.equal(error._tag, 'WorkerImportUnrecognised');
	assert.equal(error.path, WORKER_PATH);
	assert.equal(error.detail, 'cannot find the API import');
});

test('patch() reports ApiExportMissing when the worker entry drops a function', () => {
	const withoutSort = WORKER_SOURCE.replace('i as sortTailwindClasses, ', '');

	const files = new FakeTextFiles([
		[CLI_PATH, CLI_SOURCE],
		[WORKER_PATH, withoutSort],
	]);

	const error = unwrapErr(
		new OxfmtCliPatcher(files).patch(DIST_DIR),
	);

	assert.ok(error instanceof ApiExportMissing);
	assert.equal(error._tag, 'ApiExportMissing');
	assert.equal(error.role, 'sortTailwindClasses');
	assert.equal(error.path, WORKER_PATH);
});

test('patch() reports CliAnchorMissing when the Tinypool import is gone', () => {
	const files = pristineInstall();

	files.files.set(CLI_PATH, CLI_SOURCE.replace(`${TINYPOOL_IMPORT}\n`, ''));

	const error = unwrapErr(
		new OxfmtCliPatcher(files).patch(DIST_DIR),
	);

	assert.ok(error instanceof CliAnchorMissing);
	assert.equal(error._tag, 'CliAnchorMissing');
	assert.equal(error.anchor, TINYPOOL_IMPORT);
	assert.equal(error.path, CLI_PATH);
});

test('patch() reports CliAnchorMissing when the worker-proxy region marker is gone', () => {
	const files = pristineInstall();

	files.files.set(CLI_PATH, CLI_SOURCE.replace(`${REGION_MARKER}\n`, ''));

	const error = unwrapErr(
		new OxfmtCliPatcher(files).patch(DIST_DIR),
	);

	assert.ok(error instanceof CliAnchorMissing);
	assert.equal(error._tag, 'CliAnchorMissing');
	assert.equal(error.anchor, REGION_MARKER);
	assert.equal(error.path, CLI_PATH);
});

test('patch() reports CliAnchorMissing when the child_process runtime is gone', () => {
	const files = pristineInstall();

	files.files.set(CLI_PATH, CLI_SOURCE.replace(RUNTIME_ANCHOR, 'runtime: "worker_threads"'));

	const error = unwrapErr(
		new OxfmtCliPatcher(files).patch(DIST_DIR),
	);

	assert.ok(error instanceof CliAnchorMissing);
	assert.equal(error._tag, 'CliAnchorMissing');
	assert.equal(error.anchor, RUNTIME_ANCHOR);
	assert.equal(error.path, CLI_PATH);
});

test('patch() reports CliAnchorMissing when the region never closes', () => {
	const files = pristineInstall();

	files.files.set(CLI_PATH, CLI_SOURCE.replace('//#endregion\n', ''));

	const error = unwrapErr(
		new OxfmtCliPatcher(files).patch(DIST_DIR),
	);

	assert.ok(error instanceof CliAnchorMissing);
	assert.equal(error._tag, 'CliAnchorMissing');
	assert.equal(error.anchor, 'the worker-proxy region');
	assert.equal(error.path, CLI_PATH);
});

test('patch() reports CliPatchIncomplete when worker-pool code survives outside the region', () => {
	const files = pristineInstall();

	files.files.set(CLI_PATH, `${CLI_SOURCE}const leftover = Tinypool;\n`);

	const error = unwrapErr(
		new OxfmtCliPatcher(files).patch(DIST_DIR),
	);

	assert.ok(error instanceof CliPatchIncomplete);
	assert.equal(error._tag, 'CliPatchIncomplete');
	assert.equal(error.residue, 'Tinypool');
	assert.equal(error.path, CLI_PATH);
});

test('patch() reports OxfmtFileUnwritable when the rewritten CLI cannot be written back', () => {
	const files = new FakeTextFiles(
		[
			[CLI_PATH, CLI_SOURCE],
			[WORKER_PATH, WORKER_SOURCE],
		],
		[CLI_PATH],
	);

	const error = unwrapErr(
		new OxfmtCliPatcher(files).patch(DIST_DIR),
	);

	assert.ok(error instanceof OxfmtFileUnwritable);
	assert.equal(error._tag, 'OxfmtFileUnwritable');
	assert.equal(error.path, CLI_PATH);

	// The pristine CLI is still in place; nothing was half-written.
	assert.equal(files.files.get(CLI_PATH), CLI_SOURCE);
});
