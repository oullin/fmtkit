/**
 * Rewrites oxfmt's CLI so the release binary formats embedded code in-process
 * instead of through a worker pool that cannot survive `bun build --compile`.
 *
 * See `OxfmtCliPatcher` for the failure this exists to prevent.
 */
export { ApiBindings } from '#oxfmt-inprocess/api-bindings';
export { OxfmtCliPatcher } from '#oxfmt-inprocess/cli-patcher';
export type { PatchOutcome } from '#oxfmt-inprocess/cli-patcher';
export { ApiExportMissing, CliAlreadyPatched, CliAnchorMissing, CliPatchIncomplete, OxfmtFileUnreadable, OxfmtFileUnwritable, WorkerImportUnrecognised } from '#oxfmt-inprocess/errors';
export type { OxfmtPatchError } from '#oxfmt-inprocess/errors';
export { PatchCliDto } from '#oxfmt-inprocess/patch-cli-dto';
export { err, isErr, ok } from '#oxfmt-inprocess/result';
export type { Result } from '#oxfmt-inprocess/result';
export { SHIM_MARKER, ShimSource } from '#oxfmt-inprocess/shim-source';
export { NodeTextFiles } from '#oxfmt-inprocess/text-files';
export type { TextFiles } from '#oxfmt-inprocess/text-files';
