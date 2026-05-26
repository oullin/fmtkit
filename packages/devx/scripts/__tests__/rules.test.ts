import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { processSegment } from '#devx/segment';

interface Case {
	name: string;
	input: string;
	expected: string;
}

const cases: Case[] = [
	{
		name: 'import followed by export function gets a blank line',
		input: ['import { foo } from "node:foo";', 'export function bar() {', '\treturn foo();', '}', ''].join('\n'),
		expected: ['import { foo } from "node:foo";', '', 'export function bar() {', '\treturn foo();', '}', ''].join('\n'),
	},
	{
		name: 'multiple imports get a blank line only after the last one',
		input: ['import { a } from "node:a";', 'import { b } from "node:b";', 'import { c } from "node:c";', 'export function run() {', '\treturn a(b(c()));', '}', ''].join('\n'),
		expected: ['import { a } from "node:a";', 'import { b } from "node:b";', 'import { c } from "node:c";', '', 'export function run() {', '\treturn a(b(c()));', '}', ''].join('\n'),
	},
	{
		name: 'consecutive imports stay tight',
		input: ['import { a } from "node:a";', 'import { b } from "node:b";', '', 'export function run() {', '\treturn a(b());', '}', ''].join('\n'),
		expected: ['import { a } from "node:a";', 'import { b } from "node:b";', '', 'export function run() {', '\treturn a(b());', '}', ''].join('\n'),
	},
	{
		name: 'TS enum followed by function gets blank lines on both sides',
		input: ['import { x } from "node:x";', 'enum Colour {', '\tRed,', '\tBlue,', '}', 'function paint() {', '\treturn Colour.Red;', '}', ''].join('\n'),
		expected: ['import { x } from "node:x";', '', 'enum Colour {', '\tRed,', '\tBlue,', '}', '', 'function paint() {', '\treturn Colour.Red;', '}', ''].join('\n'),
	},
	{
		name: 'TS namespace (module) followed by function gets blank lines on both sides',
		input: ['namespace Utils {', '\texport const value = 1;', '}', 'function consume() {', '\treturn Utils.value;', '}', ''].join('\n'),
		expected: ['namespace Utils {', '\texport const value = 1;', '}', '', 'function consume() {', '\treturn Utils.value;', '}', ''].join('\n'),
	},
];

describe('blank-line rules', () => {
	for (const c of cases) {
		it(c.name, () => {
			const virtualName = c.name.endsWith('.js') ? 'fixture.js' : 'fixture.ts';
			const out = processSegment(c.input, virtualName);

			assert.equal(out, c.expected);
		});
	}

	it('is idempotent: running twice produces no further changes', () => {
		const input = ['import { a } from "node:a";', 'import { b } from "node:b";', 'export function run() {', '\treturn a(b());', '}', ''].join('\n');

		const once = processSegment(input, 'fixture.ts');
		const twice = processSegment(once, 'fixture.ts');

		assert.equal(twice, once);
	});
});
