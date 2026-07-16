import type { ApiBindings } from '#oxfmt-inprocess/api-bindings';

/** Marks a rewritten CLI, so a second run is reported rather than nested. */
export const SHIM_MARKER = 'fmtkit: in-process external formatter';

/**
 * Renders the source that replaces oxfmt's worker-pool proxy.
 *
 * The generated code keeps the original function names and their
 * Promise-returning contract, so the rest of oxfmt's CLI — the `runCli` wiring
 * that receives these as callbacks, and the `disposeExternalFormatter` call on
 * the way out — is left untouched.
 */
export class ShimSource {
	private constructor() {}

	/**
	 * Render the import that replaces oxfmt's `tinypool` import.
	 *
	 * @param bindings - Where the API functions live and what they are exported as.
	 * @returns An import statement binding the four API functions to shim-local names.
	 */
	static importStatement(bindings: ApiBindings): string {
		const named = [
			`${bindings.formatFile} as __fmtkitFormatFile`,
			`${bindings.formatEmbeddedCode} as __fmtkitFormatEmbeddedCode`,
			`${bindings.formatEmbeddedDoc} as __fmtkitFormatEmbeddedDoc`,
			`${bindings.sortTailwindClasses} as __fmtkitSortTailwindClasses`,
		].join(', ');

		return `import { ${named} } from "${bindings.moduleSpecifier}";`;
	}

	/**
	 * Render the replacement for oxfmt's `worker-proxy` region.
	 *
	 * The API functions are `async`, so calling one returns a Promise — exactly
	 * what `pool.run()` returned before. That Promise is handed to
	 * `toFormatFileResult`/`toNullable` unawaited on purpose: those helpers are
	 * `run.then(...)`/`run.catch(...)` wrappers, so they need the Promise itself.
	 * Awaiting first would pass a plain value, break on `run.then is not a
	 * function`, and let rejections escape instead of becoming `{ ok: false }` /
	 * `null`.
	 *
	 * @returns The `worker-proxy` region, formatting embedded code in-process.
	 */
	static workerProxyRegion(): string {
		return `//#region src-js/cli/worker-proxy.ts (${SHIM_MARKER} — see infra/scripts/release/oxfmt-inprocess)
async function initExternalFormatter(numThreads) {}
async function disposeExternalFormatter() {}
function formatFile(options, code) {
	return toFormatFileResult(__fmtkitFormatFile({ options, code }));
}
function formatEmbeddedCode(options, code) {
	return toNullable(__fmtkitFormatEmbeddedCode({ options, code }));
}
function formatEmbeddedDoc(options, texts) {
	return toNullable(__fmtkitFormatEmbeddedDoc({ options, texts }));
}
function sortTailwindClasses(options, classes) {
	return toNullable(__fmtkitSortTailwindClasses({ options, classes }));
}
//#endregion`;
	}
}
