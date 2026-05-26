import { resolve } from 'node:path';
import { dirExists, listSourceFiles, processFile } from '#devx/files';

const CANDIDATE_DIRS = ['src', 'scripts'];

export async function run(): Promise<void> {
	const cwd = process.cwd();
	const check = process.argv.includes('--check');
	const targetDirs: string[] = [];

	for (const dir of CANDIDATE_DIRS) {
		const absolute = resolve(cwd, dir);

		if (await dirExists(absolute)) {
			targetDirs.push(absolute);
		}
	}

	const files = (await Promise.all(targetDirs.map(listSourceFiles))).flat();

	let changedCount = 0;

	for (const file of files) {
		const changed = await processFile(file, check);

		if (!changed) {
			continue;
		}

		changedCount++;
		console.log(`[blank-lines] ${check ? 'would change' : 'updated'} ${file}`);
	}

	if (check && changedCount > 0) {
		console.error(`[blank-lines] ${changedCount} file(s) need blank-line edits. Run "pnpm format" to fix.`);
		process.exit(1);
	}

	console.log(`[blank-lines] processed ${files.length} file(s) in ${cwd}, ${changedCount} ${check ? 'would change' : 'changed'}`);
}
