import assert from 'node:assert/strict';
import { test } from 'node:test';
import { ComplexityScanner } from '#sidecar/complexity/complexity-scanner';
import type { ScoredFunction } from '#sidecar/complexity/complexity-scanner';

/**
 * The shared fixture: one function per construct the two language lanes have
 * to agree on. Its Go twin is `shapesSource` in
 * packages/go/complexity/golang_test.go, and the expected numbers below are
 * repeated there verbatim. A construct only one language has — try/catch and
 * the ternary here, nothing on the Go side — is asserted separately.
 */
const SHARED_CONSTRUCTS = `
export function ifChain(a: number, b: number, c: number): number {
	if (a > 0) {
		return 1;
	}

	if (b > 0) {
		return 2;
	}

	if (c > 0) {
		return 3;
	}

	return 0;
}

export function elseIfLadder(a: number): string {
	if (a === 1) {
		return 'one';
	} else if (a === 2) {
		return 'two';
	} else if (a === 3) {
		return 'three';
	} else {
		return 'other';
	}
}

export function switchFour(a: string): number {
	switch (a) {
		case 'a':
			return 1;
		case 'b':
			return 2;
		case 'c':
			return 3;
		case 'd':
			return 4;
		default:
			return 0;
	}
}

export function nestedClosure(): (item: number) => number {
	return (item) => {
		if (item > 0) {
			return item;
		}

		return 0;
	};
}

export function logicalRun(a: boolean, b: boolean, c: boolean, d: boolean): boolean {
	return a && b && c && d;
}

export function mixedLogical(a: boolean, b: boolean, c: boolean): boolean {
	return (a && b) || c;
}

export function loopWithIf(items: number[]): number {
	let total = 0;

	for (const item of items) {
		if (item > 0) {
			total += item;
		}
	}

	return total;
}
`;

/** The numbers both lanes must produce for the shared shapes. */
const SHARED_SCORES: readonly (readonly [string, number, number])[] = [
	['ifChain', 4, 3],
	['elseIfLadder', 4, 4],
	['switchFour', 5, 1],
	['nestedClosure', 2, 2],
	['logicalRun', 4, 1],
	['mixedLogical', 3, 2],
	['loopWithIf', 3, 3],
];

function scan(source: string, file = 'constructs.ts'): Map<string, ScoredFunction> {
	const result = new ComplexityScanner().scan(file, source);

	assert.deepEqual(result.errors, []);

	return new Map(result.functions.map((fn) => [fn.name, fn]));
}

test('the shared shapes score the same as their Go twins', () => {
	const scored = scan(SHARED_CONSTRUCTS);

	for (const [name, cyclomatic, cognitive] of SHARED_SCORES) {
		const fn = scored.get(name);

		assert.ok(fn, `${name} was not scored`);
		assert.equal(fn.cyclomatic, cyclomatic, `${name} cyclomatic`);
		assert.equal(fn.cognitive, cognitive, `${name} cognitive`);
	}
});

test('a catch clause costs one on both metrics', () => {
	const scored = scan(`
		export function tryCatch(run: () => void): number {
			try {
				run();

				return 0;
			} catch {
				return 1;
			}
		}
	`);

	assert.equal(scored.get('tryCatch')?.cyclomatic, 2);
	assert.equal(scored.get('tryCatch')?.cognitive, 1);
});

test('a ternary counts like an if', () => {
	const scored = scan(`
		export function ternary(a: number): number {
			return a > 0 ? 1 : 0;
		}
	`);

	assert.equal(scored.get('ternary')?.cyclomatic, 2);
	assert.equal(scored.get('ternary')?.cognitive, 1);
});

test('a logical assignment counts toward cyclomatic complexity', () => {
	const scored = scan(`
		export function defaulted(a: number | null): number {
			let value = a;

			value ??= 1;

			return value;
		}
	`);

	assert.equal(scored.get('defaulted')?.cyclomatic, 2);
});

test('a labelled jump costs one cognitive point', () => {
	const scored = scan(`
		export function labelled(rows: number[][]): number {
			outer: for (const row of rows) {
				for (const cell of row) {
					if (cell > 0) {
						break outer;
					}
				}
			}

			return 0;
		}
	`);

	// for (+1) + for (+2) + if (+3) + labelled break (+1).
	assert.equal(scored.get('labelled')?.cognitive, 7);
});

test('the key carries the file and the reporting name', () => {
	const scored = scan(SHARED_CONSTRUCTS, 'src/constructs.ts');

	assert.equal(scored.get('ifChain')?.key, 'src/constructs.ts#ifChain');
	assert.equal(scored.get('ifChain')?.file, 'src/constructs.ts');
	assert.equal(scored.get('ifChain')?.line, 2);
});

test('a parse failure is reported as a file error rather than thrown', () => {
	const result = new ComplexityScanner().scan('broken.ts', 'export function (');

	assert.equal(result.functions.length, 0);
	assert.equal(result.errors.length, 1);
	assert.equal(result.errors[0]?.file, 'broken.ts');
});
