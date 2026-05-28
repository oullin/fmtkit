import { processFile } from '#devx/files';

const cwd = process.cwd();
const rawArgs = process.argv.slice(2);
const check = rawArgs.includes('--check');

const positionalPaths = rawArgs.filter((arg) => {
	return arg !== '--check';
});

function isNotFoundError(err: unknown): boolean {
	return typeof err === 'object' && err !== null && 'code' in err && err.code === 'ENOENT';
}

function isTargetFile(path: string): boolean {
	return (path.endsWith('.ts') && !path.endsWith('.d.ts')) || path.endsWith('.vue');
}

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

main().catch((err: unknown) => {
	console.error(err);
	process.exit(1);
});
