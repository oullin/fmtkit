import { join } from 'node:path';
import { ApiBindings } from '#oxfmt-inprocess/api-bindings';
import { CliAlreadyPatched, CliAnchorMissing, CliPatchIncomplete } from '#oxfmt-inprocess/errors';
import type { OxfmtPatchError } from '#oxfmt-inprocess/errors';
import { err, isErr, ok } from '#oxfmt-inprocess/result';
import type { Result } from '#oxfmt-inprocess/result';
import { SHIM_MARKER, ShimSource } from '#oxfmt-inprocess/shim-source';
import type { TextFiles } from '#oxfmt-inprocess/text-files';

/** oxfmt's import of the worker-pool library, replaced by the shim's import. */
const TINYPOOL_IMPORT = 'import Tinypool from "tinypool";';

/**
 * The structures the rewrite edits. Absent any one of them, oxfmt's CLI has
 * changed shape and the rewrite must be re-derived rather than misapplied.
 */
const ANCHORS: ReadonlyArray<string> = [TINYPOOL_IMPORT, '//#region src-js/cli/worker-proxy.ts', 'runtime: "child_process"'];

/** oxfmt's `worker-proxy` region, from its marker to the first region end. */
const WORKER_PROXY_REGION = /\/\/#region src-js\/cli\/worker-proxy\.ts[\s\S]*?\/\/#endregion/;

/** Worker-pool code that must not survive the rewrite. */
const RESIDUE: ReadonlyArray<string> = ['Tinypool', 'pool.run', 'pool = '];

/** What a successful rewrite changed. */
export type PatchOutcome = {
	/** The CLI file that was rewritten. */
	readonly cliPath: string;

	/** The API module the shim now calls directly. */
	readonly apiModuleSpecifier: string;
};

/**
 * Rewrites oxfmt's CLI to format embedded code in-process.
 *
 * oxfmt formats embedded code — Vue `<template>`/`<style>`, markdown fences,
 * HTML — by handing it to a Tinypool pool running with
 * `runtime: "child_process"`. That pool forks worker entry scripts which, in a
 * `bun build --compile` binary, exist only as virtual `/$bunfs/root/…` paths and
 * are never carved out as real files: every worker dies on startup and
 * `Tinypool.destroy()` then awaits an exit event that never fires, wedging the
 * process. Plain `.ts` never initialises the pool, which is why only files with
 * embedded code hang.
 *
 * The four pooled functions are stateless prettier calls, so the pool only ever
 * bought cross-file parallelism — not isolation. Calling them directly is
 * therefore behaviourally identical, minus the worker layer that cannot work
 * here. For the embedded-code fraction of a codebase the lost parallelism is
 * negligible, and the per-worker prettier startup cost disappears with it.
 *
 * This runs at release-staging time against a freshly installed oxfmt in a
 * throwaway workdir; the committed tree is never touched.
 */
export class OxfmtCliPatcher {
	readonly #files: TextFiles;

	/** @param files - Reads the CLI and worker entry, and writes the result back. */
	constructor(files: TextFiles) {
		this.#files = files;
	}

	/**
	 * Rewrite oxfmt's CLI in place.
	 *
	 * @param distDir - oxfmt's `dist` directory, holding `cli.js` and `cli-worker.js`.
	 * @returns What was rewritten, or the reason oxfmt no longer matches the rewrite.
	 */
	patch(distDir: string): Result<PatchOutcome, OxfmtPatchError> {
		const cliPath = join(distDir, 'cli.js');
		const workerPath = join(distDir, 'cli-worker.js');

		const cliSource = this.#files.readText(cliPath);

		if (isErr(cliSource)) {
			return cliSource;
		}

		const workerSource = this.#files.readText(workerPath);

		if (isErr(workerSource)) {
			return workerSource;
		}

		const bindings = ApiBindings.parse(workerSource.value, workerPath);

		if (isErr(bindings)) {
			return bindings;
		}

		const rewritten = OxfmtCliPatcher.#rewrite(cliSource.value, bindings.value, cliPath);

		if (isErr(rewritten)) {
			return rewritten;
		}

		const written = this.#files.writeText(cliPath, rewritten.value);

		if (isErr(written)) {
			return written;
		}

		return ok({ cliPath, apiModuleSpecifier: bindings.value.moduleSpecifier });
	}

	/**
	 * Swap the worker-pool proxy for the in-process shim.
	 *
	 * @param cliSource - The contents of oxfmt's `cli.js`.
	 * @param bindings - Where the API functions live and what they are exported as.
	 * @param cliPath - Where the CLI came from, for error reporting.
	 * @returns The rewritten CLI, or the reason it could not be rewritten safely.
	 */
	static #rewrite(cliSource: string, bindings: ApiBindings, cliPath: string): Result<string, CliAlreadyPatched | CliAnchorMissing | CliPatchIncomplete> {
		if (cliSource.includes(SHIM_MARKER)) {
			return err(new CliAlreadyPatched(cliPath));
		}

		for (const anchor of ANCHORS) {
			if (!cliSource.includes(anchor)) {
				return err(new CliAnchorMissing(anchor, cliPath));
			}
		}

		const imported = cliSource.replace(TINYPOOL_IMPORT, ShimSource.importStatement(bindings));

		if (!WORKER_PROXY_REGION.test(imported)) {
			return err(new CliAnchorMissing('the worker-proxy region', cliPath));
		}

		const patched = imported.replace(WORKER_PROXY_REGION, ShimSource.workerProxyRegion());

		for (const residue of RESIDUE) {
			if (patched.includes(residue)) {
				return err(new CliPatchIncomplete(residue, cliPath));
			}
		}

		return ok(patched);
	}
}
