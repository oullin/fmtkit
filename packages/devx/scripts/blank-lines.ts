import { spawnSync } from 'node:child_process';
import { stat } from 'node:fs/promises';
import { isAbsolute, resolve } from 'node:path';
import { processFile } from '#devx/files';

const cwd = process.cwd();
const rawArgs = process.argv.slice(2);
const check = rawArgs.includes('--check');

const positionalPaths = rawArgs.filter((arg) => {
	return arg !== '--check';
});

function isTargetFile(path: string): boolean {
	return (path.endsWith('.ts') && !path.endsWith('.d.ts')) || path.endsWith('.vue');
}

function isNotFoundError(err: unknown): boolean {
	return typeof err === 'object' && err !== null && 'code' in err && err.code === 'ENOENT';
}

function runGitLsFiles(scope?: string): string[] {
	const args = ['ls-files', '--cached', '--others', '--exclude-standard', '-z'];

	if (scope) {
		args.push('--', scope);
	}

	const result = spawnSync('git', args, { cwd, encoding: 'buffer', maxBuffer: 64 * 1024 * 1024 });

	if (result.error || result.status !== 0) {
		const reason = result.error ? result.error.message : `git ls-files exited ${result.status}`;

		console.warn(`[blank-lines] could not list files via git (${reason}); skipping`);

		return [];
	}

	const stdout = result.stdout?.toString('utf8') ?? '';

	const entries = stdout.split('\0').filter((entry) => {
		return entry.length > 0;
	});

	const files: string[] = [];

	for (const entry of entries) {
		if (!isTargetFile(entry)) {
			continue;
		}

		files.push(isAbsolute(entry) ? entry : resolve(cwd, entry));
	}

	return files;
}

async function collectFiles(positional: string[]): Promise<string[]> {
	if (positional.length === 0) {
		return runGitLsFiles();
	}

	const collected: string[] = [];

	for (const raw of positional) {
		const absolute = isAbsolute(raw) ? raw : resolve(cwd, raw);

		const info = await stat(absolute).catch(() => {
			return null;
		});

		if (!info) {
			console.warn(`[blank-lines] path not found, skipping: ${absolute}`);

			continue;
		}

		if (info.isFile() || info.isDirectory()) {
			collected.push(...runGitLsFiles(absolute));
		}
	}

	return collected;
}

async function main(): Promise<void> {
	const files = [...new Set(await collectFiles(positionalPaths))];

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
