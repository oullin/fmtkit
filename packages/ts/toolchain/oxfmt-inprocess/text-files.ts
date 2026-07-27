import { readFileSync, writeFileSync } from 'node:fs';
import { OxfmtFileUnreadable, OxfmtFileUnwritable } from '#oxfmt-inprocess/errors';
import { err, ok } from '#oxfmt-inprocess/result';
import type { Result } from '#oxfmt-inprocess/result';

/**
 * The whole filesystem surface the rewrite needs.
 *
 * Kept this narrow so the patcher can be driven by an in-memory fake; the
 * concrete adapter is free to be wider.
 */
export type TextFiles = {
	/**
	 * Read a UTF-8 file.
	 *
	 * @param path - The file to read.
	 * @returns The file contents, or `OxfmtFileUnreadable` when it cannot be read.
	 */
	readText(path: string): Result<string, OxfmtFileUnreadable>;

	/**
	 * Overwrite a UTF-8 file.
	 *
	 * @param path - The file to write.
	 * @param contents - The contents to write.
	 * @returns Nothing, or `OxfmtFileUnwritable` when it cannot be written.
	 */
	writeText(path: string, contents: string): Result<void, OxfmtFileUnwritable>;
};

/** Reads and writes through the real filesystem. */
export class NodeTextFiles implements TextFiles {
	/**
	 * Read a UTF-8 file from disk.
	 *
	 * @param path - The file to read.
	 * @returns The file contents, or `OxfmtFileUnreadable` when it cannot be read.
	 */
	readText(path: string): Result<string, OxfmtFileUnreadable> {
		try {
			return ok(
				readFileSync(path, 'utf8'),
			);
		} catch (cause) {
			return err(new OxfmtFileUnreadable(path, cause));
		}
	}

	/**
	 * Write a UTF-8 file to disk, replacing any existing contents.
	 *
	 * @param path - The file to write.
	 * @param contents - The contents to write.
	 * @returns Nothing, or `OxfmtFileUnwritable` when it cannot be written.
	 */
	writeText(path: string, contents: string): Result<void, OxfmtFileUnwritable> {
		try {
			writeFileSync(path, contents);

			return ok(undefined);
		} catch (cause) {
			return err(new OxfmtFileUnwritable(path, cause));
		}
	}
}
