/**
 * Rewrites oxfmt's CLI to format embedded code (Vue <template>/<style>, markdown
 * code fences, HTML, etc.) IN-PROCESS instead of through a Tinypool worker pool.
 *
 * Why: the `fmtkit` release binary is produced by `bun build --compile`. oxfmt's
 * `worker-proxy` spins up a Tinypool `runtime: "child_process"` pool whose worker
 * entry scripts (`oxfmt/dist/cli-worker.js`, `tinypool/dist/entry/process.js`)
 * only exist as virtual `/$bunfs/root/...` paths inside the compiled binary and
 * are never carved out as loadable files. Every forked worker dies on startup
 * ("Worker exited unexpectedly"), and `Tinypool.destroy()` then awaits an exit
 * event that never fires, so the process hangs forever. This only triggers for
 * files with embedded code (`.vue`, `.md`, `.html`); plain `.ts`/`.tsx` never
 * initialise the pool, which is why they were unaffected.
 *
 * The four pooled functions are pure, stateless prettier calls (see
 * oxfmt/dist/cli-worker.js -> apis-*.js). The pool only bought cross-file
 * parallelism, not isolation, so calling them directly on the main thread is
 * behaviourally identical — it just removes the (unusable) worker layer. For the
 * embedded-code fraction of a codebase the lost parallelism is negligible, and
 * we drop the per-worker prettier startup cost entirely.
 *
 * This runs at release-staging time (see stage-ts-assets.sh) against the freshly
 * `npm install`ed oxfmt in the throwaway workdir — the committed tree is never
 * touched. It asserts every structure it depends on and exits non-zero if oxfmt's
 * internals shift, so an oxfmt version bump can never silently reintroduce the
 * hang.
 *
 * usage: node patch-oxfmt-inprocess.mjs <path/to/oxfmt/dist>
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

function die(message) {
	process.stderr.write(`patch-oxfmt-inprocess: ${message}\n`);
	process.exit(1);
}

const distDir = process.argv[2];

if (!distDir) {
	die('missing argument: <path/to/oxfmt/dist>');
}

const cliPath = join(distDir, 'cli.js');
const workerPath = join(distDir, 'cli-worker.js');

let cli;
let worker;

try {
	cli = readFileSync(cliPath, 'utf8');
} catch {
	die(`cannot read ${cliPath}`);
}

try {
	worker = readFileSync(workerPath, 'utf8');
} catch {
	die(`cannot read ${workerPath} — oxfmt no longer ships a worker entry; re-check whether this patch is still needed`);
}

// The worker entry re-exports the four API functions from a hashed module, e.g.
//   import { i as sortTailwindClasses, n as formatEmbeddedDoc, r as formatFile,
//            t as formatEmbeddedCode } from "./apis-CvFX8LhR.js";
// Discover the module specifier and the export alias for each role so the shim
// keeps working when the content hash or alias letters change across releases.
const workerImport = worker.match(/import\s*\{([^}]*)\}\s*from\s*["']([^"']+)["']/);

if (!workerImport) {
	die(`cannot find the API import in ${workerPath}`);
}

const [, bindingList, apiSpecifier] = workerImport;
const roleToAlias = new Map();

for (const binding of bindingList.split(',')) {
	const match = binding.trim().match(/^([A-Za-z_$][\w$]*)\s+as\s+([A-Za-z_$][\w$]*)$/);

	if (!match) {
		die(`unexpected binding "${binding.trim()}" in ${workerPath}`);
	}

	roleToAlias.set(match[2], match[1]);
}

const requiredRoles = ['formatFile', 'formatEmbeddedCode', 'formatEmbeddedDoc', 'sortTailwindClasses'];

for (const role of requiredRoles) {
	if (!roleToAlias.has(role)) {
		die(`worker entry no longer exports "${role}"`);
	}
}

// Guard the exact structure the rewrite depends on. If any anchor is gone,
// oxfmt's CLI has changed shape and this patch must be re-derived rather than
// silently misapplied.
const anchors = ['import Tinypool from "tinypool";', '//#region src-js/cli/worker-proxy.ts', 'runtime: "child_process"'];

for (const anchor of anchors) {
	if (!cli.includes(anchor)) {
		die(`anchor not found in ${cliPath}: ${anchor}`);
	}
}

if (cli.includes('fmtkit: in-process external formatter')) {
	die(`${cliPath} is already patched`);
}

// Import the API functions the shim delegates to, reusing the worker's own
// export aliases against the discovered module specifier.
const shimImport =
	`import { ${roleToAlias.get('formatFile')} as __fmtkitFormatFile, ` +
	`${roleToAlias.get('formatEmbeddedCode')} as __fmtkitFormatEmbeddedCode, ` +
	`${roleToAlias.get('formatEmbeddedDoc')} as __fmtkitFormatEmbeddedDoc, ` +
	`${roleToAlias.get('sortTailwindClasses')} as __fmtkitSortTailwindClasses } from "${apiSpecifier}";`;

// Same function names and Promise-returning shape as the pooled originals, so
// the rest of cli.js (runCli wiring, disposeExternalFormatter call) is untouched.
const shim = `//#region src-js/cli/worker-proxy.ts (fmtkit: in-process external formatter — no worker pool; see infra/scripts/release/patch-oxfmt-inprocess.mjs)
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

// Swap the Tinypool import for the API import.
let patched = cli.replace('import Tinypool from "tinypool";', shimImport);

// Replace the worker-proxy region (region marker → its matching endregion) with
// the in-process shim. Non-greedy so it stops at the first #endregion.
const regionRe = /\/\/#region src-js\/cli\/worker-proxy\.ts[\s\S]*?\/\/#endregion/;

if (!regionRe.test(patched)) {
	die('could not locate the worker-proxy region to replace');
}

patched = patched.replace(regionRe, shim);

if (patched.includes('Tinypool') || patched.includes('pool.run') || patched.includes('pool = ')) {
	die('residual Tinypool reference after patching — aborting to avoid a half-patched CLI');
}

writeFileSync(cliPath, patched);

process.stderr.write(`patch-oxfmt-inprocess: rewrote ${cliPath} to format embedded code in-process (api module: ${apiSpecifier})\n`);
