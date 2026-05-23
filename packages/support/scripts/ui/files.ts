import { readdir, readFile, stat, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { processSegment } from './segment';

const VUE_SCRIPT_REGEX = /(<script\b[^>]*>)([\s\S]*?)(<\/script>)/g;

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

		if (entry.name.endsWith('.d.ts')) {
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
		const segments: { content: string; start: number; virtualName: string }[] = [];
		VUE_SCRIPT_REGEX.lastIndex = 0;
		let match: RegExpExecArray | null;

		while ((match = VUE_SCRIPT_REGEX.exec(original)) !== null) {
			const openTag = match[1];
			const content = match[2];
			const contentStart = match.index + openTag.length;
			const virtualName = `${file}.script.ts`;

			segments.push({ content, start: contentStart, virtualName });
		}

		for (const segment of [...segments].reverse()) {
			const rewritten = processSegment(segment.content, segment.virtualName);

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
		await writeFile(file, updated);
	}

	return true;
}
