/**
 * Types for the vendored oxfmt and oxlint CLI entries that sidecar.ts imports
 * through the `@vendor/*` aliases.
 *
 * Both are side-effect-only modules: importing one runs the CLI against
 * `process.argv` and exports nothing. oxfmt ships a `dist/cli.d.ts` that says
 * exactly that (`export {}`) and oxlint ships none at all, but neither is
 * reachable by a `paths` substitution — TypeScript resolves the substituted
 * `.js` file literally rather than looking for declarations beside it — so
 * declaring them here is what keeps `noImplicitAny` satisfied without a
 * suppression at the call site.
 *
 * These declarations do NOT make the `compilerOptions.paths` entries redundant:
 * `bun build` needs them to find the real files at bundle time. See the header
 * comment in sidecar.ts for the whole arrangement.
 */
declare module '@vendor/oxfmt-cli';

declare module '@vendor/oxlint-cli';
