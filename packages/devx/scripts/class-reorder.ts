import { parseSync } from 'oxc-parser';
import { collectClassBodies, getEnd, getStart } from '#devx/ast';
import { classifyMember } from '#devx/rules';
import type { Edit, Node } from '#devx/types';

function containsComment(s: string): boolean {
	return /\/\/|\/\*/.test(s);
}

function hasCommentsAroundMembers(source: string, body: Node, members: Node[]): boolean {
	const bodyStart = getStart(body);
	const bodyEnd = getEnd(body);
	const firstStart = getStart(members[0]);
	const lastEnd = getEnd(members[members.length - 1]);

	if (containsComment(source.slice(bodyStart + 1, firstStart))) {
		return true;
	}

	for (let i = 0; i < members.length - 1; i++) {
		const gap = source.slice(getEnd(members[i]), getStart(members[i + 1]));

		if (containsComment(gap)) {
			return true;
		}
	}

	return containsComment(source.slice(lastEnd, bodyEnd - 1));
}

function computeClassReorderEdit(source: string, body: Node): Edit | null {
	const members = body.body as Node[] | undefined;

	if (!Array.isArray(members) || members.length < 2) {
		return null;
	}

	const properties: Node[] = [];
	const ctors: Node[] = [];
	const methods: Node[] = [];

	for (const member of members) {
		const kind = classifyMember(member);

		if (kind === 'property') {
			properties.push(member);
		} else if (kind === 'constructor') {
			ctors.push(member);
		} else {
			methods.push(member);
		}
	}

	const desired = [...properties, ...ctors, ...methods];
	const isSameOrder = desired.every((m, i) => m === members[i]);

	if (isSameOrder) {
		return null;
	}

	const bodyStart = getStart(body);
	const bodyEnd = getEnd(body);

	if (bodyStart < 0 || bodyEnd < 0) {
		return null;
	}

	if (hasCommentsAroundMembers(source, body, members)) {
		return null;
	}

	const firstStart = getStart(members[0]);
	const prefix = source.slice(bodyStart + 1, firstStart);
	const indentMatch = prefix.match(/\n([ \t]*)$/);

	if (!indentMatch) {
		return null;
	}

	const indent = indentMatch[1];
	const memberSlices = desired.map((m) => source.slice(getStart(m), getEnd(m)));
	const lastOriginal = members[members.length - 1];
	const closing = source.slice(getEnd(lastOriginal), bodyEnd - 1);
	const replacement = `\n${indent}${memberSlices.join(`\n${indent}`)}${closing}`;

	return {
		start: bodyStart + 1,
		end: bodyEnd - 1,
		replacement,
	};
}

export function computeReorderEdits(content: string, virtualName: string): Edit[] {
	const parsed = parseSync(virtualName, content) as unknown as { program: Node };
	const bodies = collectClassBodies(parsed.program);
	const edits: Edit[] = [];

	for (const body of bodies) {
		const edit = computeClassReorderEdit(content, body);

		if (edit) {
			edits.push(edit);
		}
	}

	return edits;
}
