import { pathToFileURL } from 'node:url';
import { processFile } from '#sidecar/files';
import { isNotFoundError, isTargetFile } from '#sidecar/pass-utils';

const cwd = process.cwd();
const rawArgs = process.argv.slice(2);
const check = rawArgs.includes('--check');

const positionalPaths = rawArgs.filter((arg) => {
	return arg !== '--check';
});

async function main(): Promise<void> {
	const files = positionalPaths.filter(isTargetFile);

	let changedCount = 0;

	for (const file of files) {
		const changed = await processFile(file, check).catch((err: unknown) => {
			if (isNotFoundError(err)) {
				console.warn(`[blank-lines] path not found, skipping: ${file}`);

				return false;
			}

			throw err;
		});

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

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	main().catch((err: unknown) => {
		console.error(err);
		process.exit(1);
	});
}
