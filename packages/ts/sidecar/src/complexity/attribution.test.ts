import assert from 'node:assert/strict';
import { test } from 'node:test';
import { ComplexityScanner } from '#sidecar/complexity/complexity-scanner';
import type { ScoredFunction } from '#sidecar/complexity/complexity-scanner';

function scan(source: string): Map<string, ScoredFunction> {
	const result = new ComplexityScanner().scan('shapes.ts', source);

	assert.deepEqual(result.errors, []);

	return new Map(result.functions.map((fn) => [fn.name, fn]));
}

test('every function-like shape reports under the name that declares it', () => {
	const scored = scan(`
		function declared(): void {}

		const assigned = function (): void {};

		const arrow = (): void => {};

		class Widget {
			constructor() {}

			method(): void {}

			get size(): number {
				return 0;
			}

			set size(value: number) {}

			field = (): void => {};
		}

		const literal = {
			property(): void {},
		};

		let later: () => void;

		later = function (): void {};
	`);

	assert.deepEqual(
		[...scored.keys()].sort(),
		['Widget.constructor', 'Widget.field', 'Widget.get size', 'Widget.method', 'Widget.set size', 'arrow', 'assigned', 'declared', 'later', 'property'].sort(),
	);
});

test('an anonymous callback folds its cognitive cost into the named owner', () => {
	const scored = scan(`
		export function owner(rows: number[][]): number[][] {
			return rows.map((row) => {
				return row.filter((cell) => {
					if (cell > 0) {
						return true;
					}

					return false;
				});
			});
		}
	`);

	assert.deepEqual([...scored.keys()], ['owner']);

	// Two closures deepen the nesting; the if then costs 1 + 2.
	assert.equal(scored.get('owner')?.cognitive, 3);
});

test('a callback keeps its own cyclomatic number and the key reports the worst', () => {
	const scored = scan(`
		export function owner(rows: number[]): number[] {
			return rows.filter((cell) => {
				if (cell > 0 && cell < 10) {
					return true;
				}

				return false;
			});
		}
	`);

	// The declaration itself branches nowhere; its callback scores 1 + if + &&.
	assert.equal(scored.get('owner')?.cyclomatic, 3);
});

test('a nested named function is reported on its own as well as folded in', () => {
	const scored = scan(`
		export function outer(a: number): number {
			function inner(b: number): number {
				if (b > 0) {
					return b;
				}

				return 0;
			}

			return inner(a);
		}
	`);

	assert.equal(scored.get('inner')?.cognitive, 1);
	assert.equal(scored.get('outer')?.cognitive, 2);
});

test('two declarations sharing a name are kept apart by their line', () => {
	const scored = scan(`
		const first = {
			handler(): void {},
		};

		const second = {
			handler(): void {},
		};
	`);

	assert.deepEqual([...scored.keys()].sort(), ['handler', 'handler:7']);
});

test('an anonymous export default reports under default', () => {
	const scored = scan(`
		export default function (value: number): number {
			return value;
		}
	`);

	assert.ok(scored.has('default'));
});

test('a function no declaration reaches reports under <anonymous>', () => {
	const scored = scan(`
		[1].map(function (value) {
			return value > 0 ? 1 : 0;
		});
	`);

	assert.equal(scored.get('<anonymous>')?.cyclomatic, 2);
});
