import type { OxcError } from 'oxc-parser';

function hasCode(cause: unknown, code: string): boolean {
	return typeof cause === 'object' && cause !== null && 'code' in cause && cause.code === code;
}

/** Source text could not be parsed into a trustworthy syntax tree. */
export class SourceUnparsable extends Error {
	readonly _tag = 'SourceUnparsable';
	readonly virtualName: string;
	readonly errors: readonly OxcError[];

	/**
	 * @param virtualName - The filename supplied to the parser.
	 * @param errors - The parser diagnostics, including messages and codeframes.
	 */
	constructor(virtualName: string, errors: readonly OxcError[]) {
		super(`cannot parse ${virtualName}`);

		this.virtualName = virtualName;
		this.errors = errors;
	}
}

/** A source file could not be read. */
export class SourceFileUnreadable extends Error {
	readonly _tag = 'SourceFileUnreadable';
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

	/**
	 * Report whether the read failed because the path does not exist.
	 *
	 * @returns `true` when the underlying filesystem error is `ENOENT`.
	 */
	isNotFound(): boolean {
		return hasCode(this.cause, 'ENOENT');
	}
}

/** A source file could not be atomically written. */
export class SourceFileUnwritable extends Error {
	readonly _tag = 'SourceFileUnwritable';
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

/** The external oxfmt process could not start or exited unsuccessfully. */
export class OxfmtRunFailed extends Error {
	readonly _tag = 'OxfmtRunFailed';
	readonly bin: string;
	readonly code: number | null;
	readonly signal: NodeJS.Signals | null;
	override readonly cause: unknown;

	/**
	 * @param bin - The oxfmt executable that was invoked.
	 * @param code - Its non-zero exit code, or `null` when unavailable.
	 * @param signal - Its terminating signal, or `null` when unavailable.
	 * @param cause - The process-spawn error, when the executable could not start.
	 */
	constructor(bin: string, code: number | null, signal: NodeJS.Signals | null, cause: unknown) {
		super(`[format-all] oxfmt exited with ${signal ?? code ?? 'an error'}`);

		this.bin = bin;
		this.code = code;
		this.signal = signal;
		this.cause = cause;
	}
}

/** A command-line argument appeared outside a recognised option or file section. */
export class UnexpectedCliArgument extends Error {
	readonly _tag = 'UnexpectedCliArgument';
	readonly arg: string;

	/** @param arg - The unexpected command-line argument. */
	constructor(arg: string) {
		super(`[format-all] unexpected argument: ${arg}`);

		this.arg = arg;
	}
}
