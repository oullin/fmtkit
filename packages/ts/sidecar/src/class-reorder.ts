import { childNodes, collectClassBodies, getEnd, getStart } from '#sidecar/ast';
import { parseCleanly } from '#sidecar/pass-utils';
import { classifyMember } from '#sidecar/rules';
import type { Edit, Node } from '#sidecar/types';

function containsComment(s: string): boolean {
	return /\/\/|\/\*/.test(s);
}

function hasCommentsAroundMembers(source: string, body: Node, members: Node[]): boolean {
	const first = members[0];
	const last = members[members.length - 1];

	if (!first || !last) {
		return false;
	}

	const bodyStart = getStart(body);
	const bodyEnd = getEnd(body);
	const firstStart = getStart(first);
	const lastEnd = getEnd(last);

	if (containsComment(
		source.slice(bodyStart + 1, firstStart),
	)) {
		return true;
	}

	for (let i = 0; i < members.length - 1; i++) {
		const current = members[i];
		const following = members[i + 1];

		if (!current || !following) {
			continue;
		}

		const gap = source.slice(getEnd(current), getStart(following));

		if (containsComment(gap)) {
			return true;
		}
	}

	return containsComment(
		source.slice(lastEnd, bodyEnd - 1),
	);
}

function computeClassReorderEdit(source: string, body: Node): Edit | null {
	const members = childNodes(body, 'body');

	if (members.length < 2) {
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

	const isSameOrder = desired.every((m, i) => {
		return m === members[i];
	});

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

	const firstMember = members[0];
	const lastOriginal = members[members.length - 1];

	if (!firstMember || !lastOriginal) {
		return null;
	}

	const firstStart = getStart(firstMember);
	const prefix = source.slice(bodyStart + 1, firstStart);
	const indentMatch = prefix.match(/\n([ \t]*)$/);

	if (!indentMatch) {
		return null;
	}

	const indent = indentMatch[1];

	const memberSlices = desired.map((m) => {
		return source.slice(getStart(m), getEnd(m));
	});

	const closing = source.slice(getEnd(lastOriginal), bodyEnd - 1);
	const replacement = `\n${indent}${memberSlices.join(`\n${indent}`)}${closing}`;

	return {
		start: bodyStart + 1,
		end: bodyEnd - 1,
		replacement,
	};
}

export function computeReorderEdits(content: string, virtualName: string): Edit[] {
	const parsed = parseCleanly(virtualName, content);

	if (!parsed) {
		return [];
	}

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
