import { isErr, ok } from '#sidecar/kernel/result';
import type { Result } from '#sidecar/kernel/result';
import type { SourceFileError, SourceFiles } from '#sidecar/io/source-files';

/** Whether an edit checks source or writes its computed changes. */
export type EditMode = 'check' | 'write';

/** Reads a file, applies a text transform, and writes only when it changed. */
export class SourceFileEditor {
	readonly #sourceFiles: SourceFiles;

	/**
	 * @param dependencies - The filesystem port used by the editor.
	 * @param dependencies.sourceFiles - Reads and atomically writes source files.
	 */
	constructor(dependencies: { sourceFiles: SourceFiles }) {
		this.#sourceFiles = dependencies.sourceFiles;
	}

	/**
	 * Read a file, transform its text, and write back only a genuine change.
	 *
	 * @param path - The source file to edit.
	 * @param mode - Whether to check or atomically write changes.
	 * @param transform - The pure text rewrite applied to the file contents.
	 * @returns Whether the file changes, or the typed filesystem failure.
	 */
	async apply(path: string, mode: EditMode, transform: (content: string) => string): Promise<Result<boolean, SourceFileError>> {
		const read = await this.#sourceFiles.readText(path);

		if (isErr(read)) {
			return read;
		}

		const original = read.value;
		const updated = transform(original);

		if (updated === original) {
			return ok(false);
		}

		if (mode === 'write') {
			const written = await this.#sourceFiles.writeTextAtomic(path, updated);

			if (isErr(written)) {
				return written;
			}
		}

		return ok(true);
	}
}
