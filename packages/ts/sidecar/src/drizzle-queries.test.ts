import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { DrizzleQueries } from '#sidecar/drizzle-queries';
import { FluentChains } from '#sidecar/fluent-chains';

describe('Drizzle query formatter', () => {
	it('formats nested where predicates after fluent-chain splitting', () => {
		const input = [
			"import { and, desc, eq, gt } from 'drizzle-orm';",
			'const rows = await db.select().from(sessions).where(and(eq(sessions.userId, userId), gt(sessions.expiresAt, now))).orderBy(desc(sessions.createdAt));',
			'',
		].join('\n');

		const expected = [
			"import { and, desc, eq, gt } from 'drizzle-orm';",
			'const rows = await db.select()',
			'\t.from(sessions)',
			'\t.where(',
			'\t\tand(',
			'\t\t\teq(sessions.userId, userId),',
			'\t\t\tgt(sessions.expiresAt, now),',
			'\t\t),',
			'\t)',
			'\t.orderBy(desc(sessions.createdAt));',
			'',
		].join('\n');

		assert.equal(FluentChains.format(input, 'fixture.ts'), expected);
	});

	it('formats join predicates with Drizzle helpers', () => {
		const input = [
			"import { and, eq, isNull } from 'drizzle-orm';",
			'const rows = await db.select().from(events).leftJoin(users, and(eq(events.userId, users.id), isNull(users.deletedAt)));',
			'',
		].join('\n');

		const expected = [
			"import { and, eq, isNull } from 'drizzle-orm';",
			'const rows = await db.select()',
			'\t.from(events)',
			'\t.leftJoin(',
			'\t\tusers,',
			'\t\tand(',
			'\t\t\teq(events.userId, users.id),',
			'\t\t\tisNull(users.deletedAt),',
			'\t\t),',
			'\t);',
			'',
		].join('\n');

		assert.equal(FluentChains.format(input, 'fixture.ts'), expected);
	});

	it('formats mutation objects and nested conflict predicates', () => {
		const input = [
			"import { and, eq } from 'drizzle-orm';",
			'await db.insert(users).values({ id: user.id, email: user.email }).onConflictDoUpdate({ target: users.id, set: { email: user.email, updatedAt: now }, where: and(eq(users.id, user.id), eq(users.active, true)) });',
			'',
		].join('\n');

		const expected = [
			"import { and, eq } from 'drizzle-orm';",
			'await db.insert(users)',
			'\t.values(',
			'\t\t{',
			'\t\t\tid: user.id,',
			'\t\t\temail: user.email,',
			'\t\t},',
			'\t)',
			'\t.onConflictDoUpdate(',
			'\t\t{',
			'\t\t\ttarget: users.id,',
			'\t\t\tset: {',
			'\t\t\t\temail: user.email,',
			'\t\t\t\tupdatedAt: now,',
			'\t\t\t},',
			'\t\t\twhere: and(',
			'\t\t\t\teq(users.id, user.id),',
			'\t\t\t\teq(users.active, true),',
			'\t\t\t),',
			'\t\t},',
			'\t);',
			'',
		].join('\n');

		assert.equal(FluentChains.format(input, 'fixture.ts'), expected);
	});

	it('formats relational query builder option objects', () => {
		const input = [
			"import { eq } from 'drizzle-orm';",
			'const users = await db.query.users.findMany({ with: { posts: { with: { comments: true } } }, where: { OR: [{ id: 1 }, { id: 2 }] } });',
			'',
		].join('\n');

		const expected = [
			"import { eq } from 'drizzle-orm';",
			'const users = await db.query.users.findMany(',
			'\t{',
			'\t\twith: {',
			'\t\t\tposts: {',
			'\t\t\t\twith: { comments: true },',
			'\t\t\t},',
			'\t\t},',
			'\t\twhere: {',
			'\t\t\tOR: [',
			'\t\t\t\t{ id: 1 },',
			'\t\t\t\t{ id: 2 },',
			'\t\t\t],',
			'\t\t},',
			'\t},',
			');',
			'',
		].join('\n');

		assert.equal(FluentChains.format(input, 'fixture.ts'), expected);
	});

	it('formats set-operation operands', () => {
		const input = ["import { union } from 'drizzle-orm';", 'const rows = await union(db.select().from(users), db.select().from(admins)).limit(10);', ''].join('\n');

		const expected = ["import { union } from 'drizzle-orm';", 'const rows = await union(', '\tdb.select().from(users),', '\tdb.select().from(admins),', ').limit(10);', ''].join('\n');

		assert.equal(FluentChains.format(input, 'fixture.ts'), expected);
	});

	it('supports aliased Drizzle imports', () => {
		const input = ["import { and as all, eq } from 'drizzle-orm';", 'const rows = await tx.select().from(users).where(all(eq(users.id, id), eq(users.active, true)));', ''].join('\n');

		const expected = [
			"import { and as all, eq } from 'drizzle-orm';",
			'const rows = await tx.select()',
			'\t.from(users)',
			'\t.where(',
			'\t\tall(',
			'\t\t\teq(users.id, id),',
			'\t\t\teq(users.active, true),',
			'\t\t),',
			'\t);',
			'',
		].join('\n');

		assert.equal(FluentChains.format(input, 'fixture.ts'), expected);
	});

	it('leaves non-Drizzle helpers unchanged', () => {
		const input = ['const rows = await db.select().from(users).where(and(eq(users.id, id), eq(users.active, true)));', ''].join('\n');

		const expected = ['const rows = await db.select()', '\t.from(users)', '\t.where(and(eq(users.id, id), eq(users.active, true)));', ''].join('\n');

		assert.equal(FluentChains.format(input, 'fixture.ts'), expected);
	});

	it('does not treat non-db select chains as Drizzle receivers', () => {
		const input = ["import { and, eq } from 'drizzle-orm';", 'const rows = await builder.select().from(users).where(and(eq(users.id, id), eq(users.active, true)));', ''].join('\n');

		const expected = ["import { and, eq } from 'drizzle-orm';", 'const rows = await builder.select()', '\t.from(users)', '\t.where(and(eq(users.id, id), eq(users.active, true)));', ''].join('\n');

		assert.equal(FluentChains.format(input, 'fixture.ts'), expected);
	});

	it('skips commented Drizzle spans', () => {
		const input = ["import { and, eq } from 'drizzle-orm';", 'const rows = await db.select().from(users).where(and(eq(users.id, id), /* keep inline */ eq(users.active, true)));', ''].join('\n');

		const expected = [
			"import { and, eq } from 'drizzle-orm';",
			'const rows = await db.select()',
			'\t.from(users)',
			'\t.where(and(eq(users.id, id), /* keep inline */ eq(users.active, true)));',
			'',
		].join('\n');

		assert.equal(FluentChains.format(input, 'fixture.ts'), expected);
	});

	it('is idempotent for formatted Drizzle queries', () => {
		const input = [
			"import { and, eq } from 'drizzle-orm';",
			'const rows = await db.select()',
			'\t.from(users)',
			'\t.where(',
			'\t\tand(',
			'\t\t\teq(users.id, id),',
			'\t\t\teq(users.active, true),',
			'\t\t),',
			'\t);',
			'',
		].join('\n');

		assert.equal(FluentChains.format(FluentChains.format(input, 'fixture.ts'), 'fixture.ts'), input);
	});

	it('nests with four spaces when the source is space-indented', () => {
		const input = ["import { and, eq } from 'drizzle-orm';", 'function load() {', '    const rows = db.select().from(users).where(and(eq(users.id, id), eq(users.active, true)));', '}', ''].join(
			'\n',
		);

		const expected = [
			"import { and, eq } from 'drizzle-orm';",
			'function load() {',
			'    const rows = db.select().from(users).where(',
			'        and(',
			'            eq(users.id, id),',
			'            eq(users.active, true),',
			'        ),',
			'    );',
			'}',
			'',
		].join('\n');

		const output = DrizzleQueries.format(input, 'fixture.ts');

		assert.equal(output, expected);
		assert.ok(!output.includes('\t'), 'space-indented Drizzle formatting must not introduce tabs');
	});

	it('does not process declaration files', () => {
		const input = [
			"import { and, eq } from 'drizzle-orm';",
			'declare const condition: ReturnType<typeof and>;',
			'declare const rows: typeof db.select().from(users).where(and(eq(users.id, id), eq(users.active, true)));',
			'',
		].join('\n');

		assert.equal(DrizzleQueries.format(input, 'fixture.d.ts'), input);
	});
});
