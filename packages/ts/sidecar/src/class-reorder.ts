import { Ast } from '#sidecar/syntax/ast';
import { isErr } from '#sidecar/kernel/result';
import { Rules } from '#sidecar/rules';
import { Sources } from '#sidecar/syntax/sources';
import type { Edit } from '#sidecar/syntax/edits';
import type { Node } from '#sidecar/syntax/node-schema';

/** Reorders class members into the formatter's stable class shape. */
export class ClassReorder {
	static #containsComment(source: string): boolean {
		return /\/\/|\/\*/.test(source);
	}

	static #hasCommentsAroundMembers(source: string, body: Node, members: Node[]): boolean {
		const first = members[0];
		const last = members.at(-1);

		if (!first || !last) {
			return false;
		}

		const bodyStart = Ast.getStart(body);
		const bodyEnd = Ast.getEnd(body);
		const firstStart = Ast.getStart(first);
		const lastEnd = Ast.getEnd(last);

		if (ClassReorder.#containsComment(source.slice(bodyStart + 1, firstStart))) {
			return true;
		}

		for (let i = 0; i < members.length - 1; i++) {
			const current = members[i];
			const following = members[i + 1];

			if (current && following && ClassReorder.#containsComment(source.slice(Ast.getEnd(current), Ast.getStart(following)))) {
				return true;
			}
		}

		return ClassReorder.#containsComment(source.slice(lastEnd, bodyEnd - 1));
	}

	static #computeClassReorderEdit(source: string, body: Node): Edit | null {
		const members = Ast.childNodes(body, 'body');

		if (members.length < 2) {
			return null;
		}

		const properties: Node[] = [];
		const constructors: Node[] = [];
		const methods: Node[] = [];

		for (const member of members) {
			const kind = Rules.classifyMember(member);

			if (kind === 'property') {
				properties.push(member);
			} else if (kind === 'constructor') {
				constructors.push(member);
			} else {
				methods.push(member);
			}
		}

		const desired = [...properties, ...constructors, ...methods];

		if (
			desired.every((member, index) => {
				return member === members[index];
			})
		) {
			return null;
		}

		const bodyStart = Ast.getStart(body);
		const bodyEnd = Ast.getEnd(body);

		if (bodyStart < 0 || bodyEnd < 0 || ClassReorder.#hasCommentsAroundMembers(source, body, members)) {
			return null;
		}

		const firstMember = members[0];
		const lastOriginal = members.at(-1);

		if (!firstMember || !lastOriginal) {
			return null;
		}

		const prefix = source.slice(bodyStart + 1, Ast.getStart(firstMember));
		const indent = prefix.match(/\n([ \t]*)$/)?.[1];

		if (indent === undefined) {
			return null;
		}

		const memberSlices = desired.map((member) => {
			return source.slice(Ast.getStart(member), Ast.getEnd(member));
		});

		const closing = source.slice(Ast.getEnd(lastOriginal), bodyEnd - 1);

		return {
			start: bodyStart + 1,
			end: bodyEnd - 1,
			replacement: `\n${indent}${memberSlices.join(`\n${indent}`)}${closing}`,
		};
	}

	/**
	 * Compute class-member ordering edits.
	 *
	 * @param content - The source text to inspect.
	 * @param virtualName - The filename used to parse the source.
	 * @returns Class-member ordering edits, or none for invalid source.
	 */
	static computeEdits(content: string, virtualName: string): Edit[] {
		const parsed = Sources.parse(virtualName, content);

		if (isErr(parsed)) {
			return [];
		}

		const edits: Edit[] = [];

		for (const body of Ast.collectClassBodies(parsed.value.program)) {
			const edit = ClassReorder.#computeClassReorderEdit(content, body);

			if (edit) {
				edits.push(edit);
			}
		}

		return edits;
	}
}
