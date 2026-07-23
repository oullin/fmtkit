import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, it } from 'node:test';
import { fileURLToPath } from 'node:url';
import { FluentChains } from '#sidecar/fluent-chains';

const script = fileURLToPath(
	import.meta.resolve('#sidecar/fluent-chains'),
);
const tsx = fileURLToPath(
	import.meta.resolve('tsx'),
);

function run(command: string, args: string[], cwd: string): void {
	const result = spawnSync(
		command,
		args,
		{ cwd, encoding: 'utf8' },
	);

	assert.equal(result.status, 0, result.stderr || result.stdout);
}

async function withFixture(files: Record<string, string>, fn: (dir: string) => Promise<void>): Promise<void> {
	const dir = await mkdtemp(
		join(
			tmpdir(),
			'fmtkit-fluent-chains-',
		),
	);

	try {
		for (const [file, content] of Object.entries(files)) {
			await writeFile(
				join(dir, file),
				content,
			);
		}

		await fn(dir);
	} finally {
		await rm(
			dir,
			{ recursive: true, force: true },
		);
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

		assert.equal(FluentChains.format(input, 'fixture.ts'), expected);
	});

	it('is idempotent for already split chains', () => {
		const input = ['const routes = createRouter()', "\t.use('*', bindEnv)", "\t.get('/', getMe);", ''].join('\n');

		assert.equal(FluentChains.format(FluentChains.format(input, 'fixture.ts'), 'fixture.ts'), input);
	});

	it('uses the file indentation style for split chains', () => {
		const input = ['function routes() {', "  return createRouter().use('*', bindEnv).get('/', getMe);", '}', ''].join('\n');
		const expected = ['function routes() {', '  return createRouter()', "    .use('*', bindEnv)", "    .get('/', getMe);", '}', ''].join('\n');

		assert.equal(FluentChains.format(input, 'fixture.ts'), expected);
	});

	it('splits chains with four spaces when the source is space-indented', () => {
		const input = ['function routes() {', "    return createRouter().use('*', bindEnv).get('/', getMe);", '}', ''].join('\n');
		const expected = ['function routes() {', '    return createRouter()', "        .use('*', bindEnv)", "        .get('/', getMe);", '}', ''].join('\n');
		const output = FluentChains.format(input, 'fixture.ts');

		assert.equal(output, expected);
		assert.ok(!output.includes('\t'), 'space-indented chain splitting must not introduce tabs');
	});

	it('indents split chains one unit past a baseline-indented block', () => {
		// An embedded block (HTML <script>, list-nested fence) whose whole body
		// sits below column zero: the baseline (3 tabs) is one nesting level, so
		// continuations land at 4 tabs, not 6.
		const input = ['\t\t\tconst result = builder().withA(1).withB(2).withC(3).build();', ''].join('\n');
		const expected = ['\t\t\tconst result = builder()', '\t\t\t\t.withA(1)', '\t\t\t\t.withB(2)', '\t\t\t\t.withC(3)', '\t\t\t\t.build();', ''].join('\n');
		const output = FluentChains.format(input, 'fixture.ts');

		assert.equal(output, expected);

		assert.ok(!output.includes('\t\t\t\t\t'), 'continuations must be base plus one unit, never doubled');
	});

	it('leaves short value transform chains unchanged', () => {
		const input = ['const normalized = value.trim().toLowerCase();', ''].join('\n');

		assert.equal(FluentChains.format(input, 'fixture.ts'), input);
	});

	it('preserves optional chain operators', () => {
		const input = ["const result = makeClient()?.use(auth).get('/');", ''].join('\n');
		const expected = ['const result = makeClient()', '\t?.use(auth)', "\t.get('/');", ''].join('\n');

		assert.equal(FluentChains.format(input, 'fixture.ts'), expected);
	});

	it('skips chains with comments between links', () => {
		const input = ['const routes = createRouter()', '\t// attach middleware first', "\t.use('*', bindEnv).get('/', getMe);", ''].join('\n');

		assert.equal(FluentChains.format(input, 'fixture.ts'), input);
	});

	it('reaches a fixed point over an expanded multiline template literal', () => {
		// Issue #59: the literal's interior gained one indent unit per run, so no
		// committed byte state survived another pass.
		const interior = ['        <div>', '            <span>hello</span>', '        </div>'];
		const input = ['const Harness = defineComponent({', '    template: `', ...interior, '    `,', '});', ''].join('\n');
		const once = FluentChains.format(input, 'fixture.ts');
		const twice = FluentChains.format(once, 'fixture.ts');

		assert.equal(twice, once);

		assert.equal(FluentChains.format(twice, 'fixture.ts'), once);

		assert.ok(once.includes(interior.join('\n')), 'the literal interior must keep its original bytes');
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
				run(
					process.execPath,
					['--import', tsx, script, 'DashboardRoute.vue'],
					dir,
				);
				run(
					process.execPath,
					['--import', tsx, script, 'DashboardRoute.vue'],
					dir,
				);

				const output = await readFile(
					join(dir, 'DashboardRoute.vue'),
					'utf8',
				);

				assert.match(output, /createRouter<\{ Bindings: WorkerEnv; Variables: IdentityVariables \}>\(\)\n\t\.use\('\*', bindEnv\)\n\t\.use\(identityMiddleware\)/);
				assert.match(output, /\.get\('\/', getMe\)\n\t\.get\('\/sessions', getSessions\);/);
			},
		);
	});

	it('skips non-JavaScript Vue script blocks through the CLI', async () => {
		const jsonLd = '{"@context":"https://schema.org","@type":"WebSite","url":"https://example.com"}';

		await withFixture(
			{
				'StructuredData.vue': [
					'<script type="application/ld+json">',
					jsonLd,
					'</script>',
					'<script setup lang="ts">',
					"export const meRoutes = createRouter().use('*', bindEnv).get('/', getMe);",
					'</script>',
					'',
				].join('\n'),
			},
			async (dir) => {
				run(
					process.execPath,
					['--import', tsx, script, 'StructuredData.vue'],
					dir,
				);

				const output = await readFile(
					join(dir, 'StructuredData.vue'),
					'utf8',
				);

				assert.ok(output.includes(['<script type="application/ld+json">', jsonLd, '</script>'].join('\n')));
				assert.match(output, /createRouter\(\)\n\t\.use\('\*', bindEnv\)\n\t\.get\('\/', getMe\);/);
			},
		);
	});

	it('formats HTML script blocks through the CLI', async () => {
		await withFixture(
			{
				'index.html': [
					'<!doctype html>',
					'<body>',
					'<script>',
					"export const meRoutes = createRouter().use('*', bindEnv).use(identityMiddleware).get('/', getMe);",
					'</script>',
					'</body>',
					'',
				].join('\n'),
			},
			async (dir) => {
				run(
					process.execPath,
					['--import', tsx, script, 'index.html'],
					dir,
				);
				run(
					process.execPath,
					['--import', tsx, script, 'index.html'],
					dir,
				);

				const output = await readFile(
					join(dir, 'index.html'),
					'utf8',
				);

				assert.match(output, /createRouter\(\)\n\t\.use\('\*', bindEnv\)\n\t\.use\(identityMiddleware\)\n\t\.get\('\/', getMe\);/);

				assert.ok(output.includes('<!doctype html>') && output.includes('</body>'), 'must preserve the host HTML markup');
			},
		);
	});

	it('formats Markdown fenced blocks through the CLI', async () => {
		await withFixture(
			{
				'notes.md': [
					'# Routes',
					'',
					'```ts',
					"export const meRoutes = createRouter().use('*', bindEnv).use(identityMiddleware).get('/', getMe);",
					'```',
					'',
					'```bash',
					"echo 'left alone'",
					'```',
					'',
				].join('\n'),
			},
			async (dir) => {
				run(
					process.execPath,
					['--import', tsx, script, 'notes.md'],
					dir,
				);
				run(
					process.execPath,
					['--import', tsx, script, 'notes.md'],
					dir,
				);

				const output = await readFile(
					join(dir, 'notes.md'),
					'utf8',
				);

				assert.match(output, /createRouter\(\)\n\t\.use\('\*', bindEnv\)\n\t\.use\(identityMiddleware\)\n\t\.get\('\/', getMe\);/);

				assert.ok(output.includes("echo 'left alone'"), 'must leave the non-JS fence untouched');
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
				run(
					process.execPath,
					['--import', tsx, script, 'DashboardData.vue'],
					dir,
				);
				run(
					process.execPath,
					['--import', tsx, script, 'DashboardData.vue'],
					dir,
				);

				const output = await readFile(
					join(dir, 'DashboardData.vue'),
					'utf8',
				);

				assert.match(output, /\.where\(\n\t\tand\(\n\t\t\teq\(sessions\.userId, userId\),\n\t\t\tgt\(sessions\.expiresAt, now\),\n\t\t\),\n\t\);/);
			},
		);
	});

	it('formats expanded call arguments in Vue script blocks through the CLI', async () => {
		await withFixture(
			{
				'AuthProvider.vue': [
					'<script setup lang="ts">',
					'function createAuth() {',
					'\treturn betterAuth(buildAuthConfig(env, db, mailer, app, options)) as unknown as SasuAuth;',
					'}',
					'</script>',
					'',
				].join('\n'),
			},
			async (dir) => {
				run(
					process.execPath,
					['--import', tsx, script, 'AuthProvider.vue'],
					dir,
				);
				run(
					process.execPath,
					['--import', tsx, script, 'AuthProvider.vue'],
					dir,
				);

				const output = await readFile(
					join(dir, 'AuthProvider.vue'),
					'utf8',
				);

				assert.match(output, /return betterAuth\(\n\t\tbuildAuthConfig\(env, db, mailer, app, options\),\n\t\) as unknown as SasuAuth;/);
			},
		);
	});
});
