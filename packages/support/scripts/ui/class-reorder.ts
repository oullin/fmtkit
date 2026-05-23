import { parseSync } from 'oxc-parser';
import { collectClassBodies, getEnd, getStart } from './ast';
import { classifyMember } from './rules';
import type { Edit, Node } from './types';

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
