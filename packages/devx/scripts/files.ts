import { readdir, readFile, stat } from 'node:fs/promises';
import { resolve } from 'node:path';
import { extractVueScripts, isJavaScriptOrTypeScript, writeFileAtomic } from '#devx/pass-utils';
import { processSegment } from '#devx/segment';

export async function dirExists(dir: string): Promise<boolean> {
	try {
		const s = await stat(dir);

		return s.isDirectory();
	} catch {
		return false;
	}
}

export async function listSourceFiles(dir: string): Promise<string[]> {
	const entries = await readdir(dir, { recursive: true, withFileTypes: true });

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

export async function processFile(file: string, check: boolean): Promise<boolean> {
	const original = await readFile(file, 'utf8');

	let updated = original;

	if (file.endsWith('.vue')) {
		const segments = extractVueScripts(original).filter((segment) => {
			return isJavaScriptOrTypeScript(segment.openTag);
		});

		for (const segment of [...segments].reverse()) {
			const rewritten = processSegment(segment.content, `${file}.script.ts`);

			if (rewritten === segment.content) {
				continue;
			}

			updated = updated.slice(0, segment.start) + rewritten + updated.slice(segment.start + segment.content.length);
		}
	} else {
		updated = processSegment(original, file);
	}

	if (updated === original) {
		return false;
	}

	if (!check) {
		await writeFileAtomic(file, updated);
	}

	return true;
}
