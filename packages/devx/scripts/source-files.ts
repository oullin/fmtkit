import { spawnSync } from 'node:child_process';
import { stat } from 'node:fs/promises';
import { isAbsolute, resolve } from 'node:path';

const cwd = process.cwd();

export function isTargetFile(path: string, includeDeclarations = false): boolean {
	if (!path.endsWith('.ts') && !path.endsWith('.vue')) {
		return false;
	}

	return includeDeclarations || !path.endsWith('.d.ts');
}

export function runGitLsFiles(scope?: string, includeDeclarations = false, warningLabel = 'source-files'): string[] {
	const args = ['ls-files', '--cached', '--others', '--exclude-standard', '-z'];

	if (scope) {
		args.push('--', scope);
	}

	const result = spawnSync('git', args, { cwd, encoding: 'buffer', maxBuffer: 64 * 1024 * 1024 });

	if (result.error || result.status !== 0) {
		const reason = result.error ? result.error.message : `git ls-files exited ${result.status}`;

		console.warn(`[${warningLabel}] could not list files via git (${reason}); skipping`);

		return [];
	}

	const stdout = result.stdout?.toString('utf8') ?? '';

	const entries = stdout.split('\0').filter((entry) => {
		return entry.length > 0;
	});

	const files: string[] = [];

	for (const entry of entries) {
		if (!isTargetFile(entry, includeDeclarations)) {
			continue;
		}

		files.push(isAbsolute(entry) ? entry : resolve(cwd, entry));
	}

	return files;
}

export async function collectSourceFiles(positional: string[], includeDeclarations = false, warningLabel = 'source-files'): Promise<string[]> {
	if (positional.length === 0) {
		return runGitLsFiles(undefined, includeDeclarations, warningLabel);
	}

	const collected: string[] = [];

	for (const raw of positional) {
		const absolute = isAbsolute(raw) ? raw : resolve(cwd, raw);

		const info = await stat(absolute).catch(() => {
			return null;
		});

		if (!info) {
			console.warn(`[${warningLabel}] path not found, skipping: ${absolute}`);

			continue;
		}

		if (info.isFile() || info.isDirectory()) {
			collected.push(...runGitLsFiles(absolute, includeDeclarations, warningLabel));
		}
	}

	return [...new Set(collected)];
}
