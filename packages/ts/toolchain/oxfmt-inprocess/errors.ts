/**
 * Typed failures raised while rewriting oxfmt's CLI.
 *
 * Every failure here means the same thing operationally: oxfmt's internals no
 * longer match what the rewrite assumes, so the release must stop rather than
 * ship a half-patched CLI that hangs at runtime. Each error therefore carries
 * the exact anchor/path it was looking for, so a version bump reports what to
 * re-derive instead of a bare stack trace.
 */

/** A file the rewrite depends on could not be read. */
export class OxfmtFileUnreadable extends Error {
	readonly _tag = 'OxfmtFileUnreadable';
	readonly path: string;
	override readonly cause: unknown;

	/**
	 * @param path - The file that could not be read.
	 * @param cause - The underlying filesystem error.
	 */
	constructor(path: string, cause: unknown) {
		super(`cannot read ${path}`);

		this.path = path;
		this.cause = cause;
	}
}

/** The rewritten CLI could not be written back. */
export class OxfmtFileUnwritable extends Error {
	readonly _tag = 'OxfmtFileUnwritable';
	readonly path: string;
	override readonly cause: unknown;

	/**
	 * @param path - The file that could not be written.
	 * @param cause - The underlying filesystem error.
	 */
	constructor(path: string, cause: unknown) {
		super(`cannot write ${path}`);

		this.path = path;
		this.cause = cause;
	}
}

/** The worker entry no longer carries a recognisable API import. */
export class WorkerImportUnrecognised extends Error {
	readonly _tag = 'WorkerImportUnrecognised';
	readonly path: string;
	readonly detail: string;

	/**
	 * @param path - The worker entry that was parsed.
	 * @param detail - What about the import could not be read.
	 */
	constructor(path: string, detail: string) {
		super(`${detail} in ${path}`);

		this.path = path;
		this.detail = detail;
	}
}

/** The worker entry no longer re-exports a function the shim delegates to. */
export class ApiExportMissing extends Error {
	readonly _tag = 'ApiExportMissing';
	readonly role: string;
	readonly path: string;

	/**
	 * @param role - The function the shim expected to find.
	 * @param path - The worker entry that was parsed.
	 */
	constructor(role: string, path: string) {
		super(`worker entry ${path} no longer exports "${role}"`);

		this.role = role;
		this.path = path;
	}
}

/** oxfmt's CLI no longer contains a structure the rewrite edits. */
export class CliAnchorMissing extends Error {
	readonly _tag = 'CliAnchorMissing';
	readonly anchor: string;
	readonly path: string;

	/**
	 * @param anchor - The source fragment that was expected.
	 * @param path - The CLI file that was searched.
	 */
	constructor(anchor: string, path: string) {
		super(`anchor not found in ${path}: ${anchor}`);

		this.anchor = anchor;
		this.path = path;
	}
}

/** oxfmt's CLI already carries the in-process shim. */
export class CliAlreadyPatched extends Error {
	readonly _tag = 'CliAlreadyPatched';
	readonly path: string;

	/** @param path - The CLI file that was already rewritten. */
	constructor(path: string) {
		super(`${path} is already patched`);

		this.path = path;
	}
}

/** The rewrite left worker-pool code behind, so the result cannot be trusted. */
export class CliPatchIncomplete extends Error {
	readonly _tag = 'CliPatchIncomplete';
	readonly residue: string;
	readonly path: string;

	/**
	 * @param residue - The worker-pool fragment still present after rewriting.
	 * @param path - The CLI file that was rewritten.
	 */
	constructor(residue: string, path: string) {
		super(`residual worker-pool reference "${residue}" in ${path} after patching`);

		this.residue = residue;
		this.path = path;
	}
}

/** Every way rewriting oxfmt's CLI is expected to fail. */
export type OxfmtPatchError = ApiExportMissing | CliAlreadyPatched | CliAnchorMissing | CliPatchIncomplete | OxfmtFileUnreadable | OxfmtFileUnwritable | WorkerImportUnrecognised;
