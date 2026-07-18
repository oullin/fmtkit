import assert from 'node:assert/strict';
import { test } from 'node:test';
import { ApiBindings } from '#oxfmt-inprocess/api-bindings';
import { ApiExportMissing, WorkerImportUnrecognised } from '#oxfmt-inprocess/errors';
import { isErr } from '#oxfmt-inprocess/result';
import type { Result } from '#oxfmt-inprocess/result';

/** Where the parsed worker entry nominally came from, for error reporting. */
const WORKER_PATH = '/oxfmt/dist/cli-worker.js';

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

test('parse() reads rolldown-aliased bindings and the hashed module specifier', () => {
	const workerSource = `import { i as sortTailwindClasses, n as formatEmbeddedDoc, r as formatFile, t as formatEmbeddedCode } from './apis-CvFX8LhR.js';
export { formatEmbeddedCode, formatEmbeddedDoc, formatFile, sortTailwindClasses };
`;

	const bindings = unwrapOk(ApiBindings.parse(workerSource, WORKER_PATH));

	assert.equal(bindings.moduleSpecifier, './apis-CvFX8LhR.js');
	assert.equal(bindings.formatFile, 'r');
	assert.equal(bindings.formatEmbeddedCode, 't');
	assert.equal(bindings.formatEmbeddedDoc, 'n');
	assert.equal(bindings.sortTailwindClasses, 'i');
});

test('parse() reads bare bindings, each exported under its own name', () => {
	const workerSource = `import { formatEmbeddedCode, formatEmbeddedDoc, formatFile, sortTailwindClasses } from "./apis.js";
`;

	const bindings = unwrapOk(ApiBindings.parse(workerSource, WORKER_PATH));

	assert.equal(bindings.moduleSpecifier, './apis.js');
	assert.equal(bindings.formatFile, 'formatFile');
	assert.equal(bindings.formatEmbeddedCode, 'formatEmbeddedCode');
	assert.equal(bindings.formatEmbeddedDoc, 'formatEmbeddedDoc');
	assert.equal(bindings.sortTailwindClasses, 'sortTailwindClasses');
});

test('parse() tolerates a trailing comma in the import', () => {
	const workerSource = `import { r as formatFile, t as formatEmbeddedCode, n as formatEmbeddedDoc, i as sortTailwindClasses, } from './apis.js';
`;

	const bindings = unwrapOk(ApiBindings.parse(workerSource, WORKER_PATH));

	assert.equal(bindings.formatFile, 'r');
	assert.equal(bindings.sortTailwindClasses, 'i');
});

test('parse() reports WorkerImportUnrecognised when the entry carries no API import', () => {
	const error = unwrapErr(ApiBindings.parse('export {};\n', WORKER_PATH));

	assert.ok(error instanceof WorkerImportUnrecognised);
	assert.equal(error._tag, 'WorkerImportUnrecognised');
	assert.equal(error.path, WORKER_PATH);
	assert.equal(error.detail, 'cannot find the API import');
});

test('parse() reports WorkerImportUnrecognised for a binding it cannot read', () => {
	const workerSource = `import { r as formatFile, 1bad } from './apis.js';
`;

	const error = unwrapErr(ApiBindings.parse(workerSource, WORKER_PATH));

	assert.ok(error instanceof WorkerImportUnrecognised);
	assert.equal(error._tag, 'WorkerImportUnrecognised');
	assert.equal(error.path, WORKER_PATH);
	assert.equal(error.detail, 'unexpected binding "1bad"');
});

test('parse() reports ApiExportMissing for the function the entry no longer re-exports', () => {
	const workerSource = `import { n as formatEmbeddedDoc, r as formatFile, t as formatEmbeddedCode } from './apis.js';
`;

	const error = unwrapErr(ApiBindings.parse(workerSource, WORKER_PATH));

	assert.ok(error instanceof ApiExportMissing);
	assert.equal(error._tag, 'ApiExportMissing');
	assert.equal(error.role, 'sortTailwindClasses');
	assert.equal(error.path, WORKER_PATH);
});
