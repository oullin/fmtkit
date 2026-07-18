import { readdir, stat } from 'node:fs/promises';
import { resolve } from 'node:path';

/** Reads source-file inventories from the local filesystem. */
export class Files {
	/** Report whether a path exists as a directory. */
	static async dirExists(directory: string): Promise<boolean> {
		try {
			return (await stat(directory)).isDirectory();
		} catch {
			return false;
		}
	}

	/** List TypeScript and Vue files below a directory recursively. */
	static async listSourceFiles(directory: string): Promise<string[]> {
		const entries = await readdir(
			directory,
			{ recursive: true, withFileTypes: true },
		);

		const files: string[] = [];

		for (const entry of entries) {
			if (entry.isFile() && (entry.name.endsWith('.ts') || entry.name.endsWith('.vue'))) {
				files.push(resolve(entry.parentPath, entry.name));
			}
		}

		return files;
	}
}
