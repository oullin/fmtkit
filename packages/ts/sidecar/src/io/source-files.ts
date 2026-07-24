import { randomBytes } from 'node:crypto';
import { readFile, rename, rm, writeFile } from 'node:fs/promises';
import { SourceFileUnreadable, SourceFileUnwritable } from '#sidecar/kernel/errors';
import { err, ok } from '#sidecar/kernel/result';
import type { Result } from '#sidecar/kernel/result';

/** Every expected filesystem failure reported by the source-file port. */
export type SourceFileError = SourceFileUnreadable | SourceFileUnwritable;

/** The filesystem operations required by the formatting pipeline. */
export type SourceFiles = {
	/**
	 * Read a UTF-8 source file.
	 *
	 * @param path - The file to read.
	 * @returns Its contents, or a typed read failure.
	 */
	readText(path: string): Promise<Result<string, SourceFileUnreadable>>;

	/**
	 * Atomically replace a UTF-8 source file.
	 *
	 * @param path - The file to replace.
	 * @param content - The complete replacement contents.
	 * @returns Nothing, or a typed write failure.
	 */
	writeTextAtomic(path: string, content: string): Promise<Result<void, SourceFileUnwritable>>;
};

/** Reads source files and atomically writes them through Node's filesystem APIs. */
export class NodeSourceFiles implements SourceFiles {
	/**
	 * Read a UTF-8 source file.
	 *
	 * @param path - The file to read.
	 * @returns Its contents, or `SourceFileUnreadable` carrying the filesystem cause.
	 */
	async readText(path: string): Promise<Result<string, SourceFileUnreadable>> {
		try {
			return ok(await readFile(path, 'utf8'));
		} catch (cause) {
			return err(new SourceFileUnreadable(path, cause));
		}
	}

	/**
	 * Atomically replace a UTF-8 source file through a sibling temporary file.
	 *
	 * @param path - The file to replace.
	 * @param content - The complete replacement contents.
	 * @returns Nothing, or `SourceFileUnwritable` carrying the filesystem cause.
	 */
	async writeTextAtomic(path: string, content: string): Promise<Result<void, SourceFileUnwritable>> {
		const temporaryPath = `${path}.${process.pid}.${randomBytes(6).toString('hex')}.tmp`;

		try {
			await writeFile(temporaryPath, content);

			await rename(temporaryPath, path);

			return ok(undefined);
		} catch (cause) {
			try {
				await rm(
					temporaryPath,
					{ force: true },
				);
			} catch {
				// Ignore cleanup failures so the original write/rename error is carried.
			}

			return err(new SourceFileUnwritable(path, cause));
		}
	}
}
