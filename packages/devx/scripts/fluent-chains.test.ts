import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, it } from 'node:test';
import { fileURLToPath } from 'node:url';
import { formatFluentChains } from '#devx/fluent-chains';

const script = fileURLToPath(import.meta.resolve('#devx/fluent-chains'));
const tsx = fileURLToPath(import.meta.resolve('tsx/cli'));

function run(command: string, args: string[], cwd: string): void {
	const result = spawnSync(command, args, { cwd, encoding: 'utf8' });

	assert.equal(result.status, 0, result.stderr || result.stdout);
}

async function withFixture(files: Record<string, string>, fn: (dir: string) => Promise<void>): Promise<void> {
	const dir = await mkdtemp(join(tmpdir(), 'go-fmt-fluent-chains-'));

	try {
		for (const [file, content] of Object.entries(files)) {
			await writeFile(join(dir, file), content);
		}

		await fn(dir);
	} finally {
		await rm(dir, { recursive: true, force: true });
	}
}

describe('fluent chain formatter', () => {
	it('splits router builder chains onto independent lines', () => {
		const input = [
			"export const meRoutes = createRouter<{ Bindings: WorkerEnv; Variables: IdentityVariables }>().use('*', bindEnv).use(identityMiddleware).get('/', getMe).get('/sessions', getSessions);",
			'',
		].join('\n');

		const expected = [
			'export const meRoutes = createRouter<{ Bindings: WorkerEnv; Variables: IdentityVariables }>()',
			"\t.use('*', bindEnv)",
			'\t.use(identityMiddleware)',
			"\t.get('/', getMe)",
			"\t.get('/sessions', getSessions);",
			'',
		].join('\n');

		assert.equal(formatFluentChains(input, 'fixture.ts'), expected);
	});

	it('is idempotent for already split chains', () => {
		const input = ['const routes = createRouter()', "\t.use('*', bindEnv)", "\t.get('/', getMe);", ''].join('\n');

		assert.equal(formatFluentChains(formatFluentChains(input, 'fixture.ts'), 'fixture.ts'), input);
	});

	it('leaves short value transform chains unchanged', () => {
		const input = ['const normalized = value.trim().toLowerCase();', ''].join('\n');

		assert.equal(formatFluentChains(input, 'fixture.ts'), input);
	});

	it('preserves optional chain operators', () => {
		const input = ["const result = makeClient()?.use(auth).get('/');", ''].join('\n');
		const expected = ['const result = makeClient()', '\t?.use(auth)', "\t.get('/');", ''].join('\n');

		assert.equal(formatFluentChains(input, 'fixture.ts'), expected);
	});

	it('skips chains with comments between links', () => {
		const input = ['const routes = createRouter()', '\t// attach middleware first', "\t.use('*', bindEnv).get('/', getMe);", ''].join('\n');

		assert.equal(formatFluentChains(input, 'fixture.ts'), input);
	});

	it('formats Vue script blocks through the CLI', async () => {
		await withFixture(
			{
				'DashboardRoute.vue': [
					'<script setup lang="ts">',
					"export const meRoutes = createRouter<{ Bindings: WorkerEnv; Variables: IdentityVariables }>().use('*', bindEnv).use(identityMiddleware).get('/', getMe).get('/sessions', getSessions);",
					'</script>',
					'',
				].join('\n'),
			},
			async (dir) => {
				run(process.execPath, [tsx, script, 'DashboardRoute.vue'], dir);
				run(process.execPath, [tsx, script, 'DashboardRoute.vue'], dir);

				const output = await readFile(join(dir, 'DashboardRoute.vue'), 'utf8');

				assert.match(output, /createRouter<\{ Bindings: WorkerEnv; Variables: IdentityVariables \}>\(\)\n\t\.use\('\*', bindEnv\)\n\t\.use\(identityMiddleware\)/);
				assert.match(output, /\.get\('\/', getMe\)\n\t\.get\('\/sessions', getSessions\);/);
			},
		);
	});

	it('formats Drizzle queries in Vue script blocks through the CLI', async () => {
		await withFixture(
			{
				'DashboardData.vue': [
					'<script setup lang="ts">',
					"import { and, eq, gt } from 'drizzle-orm';",
					'const rows = await db.select().from(sessions).where(and(eq(sessions.userId, userId), gt(sessions.expiresAt, now)));',
					'</script>',
					'',
				].join('\n'),
			},
			async (dir) => {
				run(process.execPath, [tsx, script, 'DashboardData.vue'], dir);
				run(process.execPath, [tsx, script, 'DashboardData.vue'], dir);

				const output = await readFile(join(dir, 'DashboardData.vue'), 'utf8');

				assert.match(output, /\.where\(\n\t\tand\(\n\t\t\teq\(sessions\.userId, userId\),\n\t\t\tgt\(sessions\.expiresAt, now\),\n\t\t\),\n\t\);/);
			},
		);
	});
});
