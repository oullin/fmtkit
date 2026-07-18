import { readdir, stat } from 'node:fs/promises';
import { resolve } from 'node:path';

export async function dirExists(dir: string): Promise<boolean> {
	try {
		const s = await stat(dir);

		return s.isDirectory();
	} catch {
		return false;
	}
}

export async function listSourceFiles(dir: string): Promise<string[]> {
	const entries = await readdir(
		dir,
		{ recursive: true, withFileTypes: true },
	);

	const files: string[] = [];

	for (const entry of entries) {
		if (!entry.isFile()) {
			continue;
		}

		if (!entry.name.endsWith('.ts') && !entry.name.endsWith('.vue')) {
			continue;
		}

		files.push(resolve(entry.parentPath, entry.name));
	}

	return files;
}
