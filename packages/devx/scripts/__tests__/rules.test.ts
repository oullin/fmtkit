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
	{
		name: 'inline statement bodies are wrapped',
		input: ['function run() {', '\tif (a) b(); else if (c) d(); else e();', '\tfor (const item of items) consume(item);', '\tconst fn = (value: number) => value + 1;', '}', ''].join('\n'),
		expected: [
			'function run() {',
			'\tif (a) {',
			'\t\tb();',
			'\t} else if (c) {',
			'\t\td();',
			'\t} else {',
			'\t\te();',
			'\t}',
			'',
			'\tfor (const item of items) {',
			'\t\tconsume(item);',
			'\t}',
			'',
			'\tconst fn = (value: number) => value + 1;',
			'}',
			'',
		].join('\n'),
	},
	{
		name: 'nested if statement bodies are wrapped except else-if chains',
		input: ['function run() {', '\tif (a) if (b) c();', '\tfor (const item of items) if (item.ready) consume(item);', '}', ''].join('\n'),
		expected: [
			'function run() {',
			'\tif (a) {',
			'\t\tif (b) {',
			'\t\t\tc();',
			'\t\t}',
			'\t}',
			'',
			'\tfor (const item of items) {',
			'\t\tif (item.ready) {',
			'\t\t\tconsume(item);',
			'\t\t}',
			'\t}',
			'}',
			'',
		].join('\n'),
	},
	{
		name: 'await statements are isolated from adjacent code',
		input: ['async function run() {', '\tconst before = 1;', '\tawait work();', '\tconst after = 2;', '}', ''].join('\n'),
		expected: ['async function run() {', '\tconst before = 1;', '', '\tawait work();', '', '\tconst after = 2;', '}', ''].join('\n'),
	},
	{
		name: 'await inside nested functions does not isolate parent statements',
		input: ['function run() {', '\tconst onClick = async () => await work();', '\tconst after = 1;', '}', ''].join('\n'),
		expected: ['function run() {', '\tconst onClick = async () => await work();', '\tconst after = 1;', '}', ''].join('\n'),
	},
	{
		name: 'Vue primitive const declarations get a blank line above',
		input: ['function setupState() {', '\tconst before = 1;', '\tconst value = computed(() => 1);', '\tconst after = 2;', '}', ''].join('\n'),
		expected: ['function setupState() {', '\tconst before = 1;', '', '\tconst value = computed(() => 1);', '\tconst after = 2;', '}', ''].join('\n'),
	},
	{
		name: 'multiline imports and consts move last in their groups',
		input: ['import { z } from "z";', 'import {', '\ta,', '} from "a";', 'import { y } from "y";', 'const b = 1;', 'const a = {', '\tx: 1,', '};', 'const c = 2;', ''].join('\n'),
		expected: ['import { z } from "z";', 'import { y } from "y";', '', 'import {', '\ta,', '} from "a";', '', 'const b = 1;', 'const c = 2;', '', 'const a = {', '\tx: 1,', '};', ''].join('\n'),
	},
	{
		name: 'multiline consts with nested side effects keep their order',
		input: ['const config = {', '\tvalue: makeValue(),', '};', 'const next = 1;', ''].join('\n'),
		expected: ['const config = {', '\tvalue: makeValue(),', '};', '', 'const next = 1;', ''].join('\n'),
	},
	{
		name: 'multiline destructuring consts keep their order',
		input: ['const { value } = {', '\tvalue: 1,', '};', 'const next = value;', ''].join('\n'),
		expected: ['const { value } = {', '\tvalue: 1,', '};', '', 'const next = value;', ''].join('\n'),
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
