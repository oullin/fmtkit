import { AstReader } from '#sidecar/syntax/ast-reader';
import { isErr } from '#sidecar/kernel/result';
import { Rules } from '#sidecar/rules';
import { SourceParser } from '#sidecar/syntax/source-parser';
import type { Edit } from '#sidecar/syntax/edits';
import type { Node } from '#sidecar/syntax/node-schema';

/** Reorders class members into the formatter's stable class shape. */
export class ClassReorder {
	static readonly #ast = new AstReader();

	static readonly #parser = new SourceParser();

	static #containsComment(source: string): boolean {
		return /\/\/|\/\*/.test(source);
	}

	static #hasCommentsAroundMembers(source: string, body: Node, members: Node[]): boolean {
		const first = members[0];
		const last = members.at(-1);

		if (!first || !last) {
			return false;
		}

		const bodyStart = ClassReorder.#ast.getStart(body);
		const bodyEnd = ClassReorder.#ast.getEnd(body);
		const firstStart = ClassReorder.#ast.getStart(first);
		const lastEnd = ClassReorder.#ast.getEnd(last);

		if (ClassReorder.#containsComment(source.slice(bodyStart + 1, firstStart))) {
			return true;
		}

		for (let i = 0; i < members.length - 1; i++) {
			const current = members[i];
			const following = members[i + 1];

			if (current && following && ClassReorder.#containsComment(source.slice(ClassReorder.#ast.getEnd(current), ClassReorder.#ast.getStart(following)))) {
				return true;
			}
		}

		return ClassReorder.#containsComment(source.slice(lastEnd, bodyEnd - 1));
	}

	static #computeClassReorderEdit(source: string, body: Node): Edit | null {
		const members = ClassReorder.#ast.childNodes(body, 'body');

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

		const bodyStart = ClassReorder.#ast.getStart(body);
		const bodyEnd = ClassReorder.#ast.getEnd(body);

		if (bodyStart < 0 || bodyEnd < 0 || ClassReorder.#hasCommentsAroundMembers(source, body, members)) {
			return null;
		}

		const firstMember = members[0];
		const lastOriginal = members.at(-1);

		if (!firstMember || !lastOriginal) {
			return null;
		}

		const prefix = source.slice(bodyStart + 1, ClassReorder.#ast.getStart(firstMember));
		const indent = prefix.match(/\n([ \t]*)$/)?.[1];

		if (indent === undefined) {
			return null;
		}

		const memberSlices = desired.map((member) => {
			return source.slice(ClassReorder.#ast.getStart(member), ClassReorder.#ast.getEnd(member));
		});

		const closing = source.slice(ClassReorder.#ast.getEnd(lastOriginal), bodyEnd - 1);

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
		const parsed = ClassReorder.#parser.parse(virtualName, content);

		if (isErr(parsed)) {
			return [];
		}

		const edits: Edit[] = [];

		for (const body of ClassReorder.#ast.collectClassBodies(parsed.value.program)) {
			const edit = ClassReorder.#computeClassReorderEdit(content, body);

			if (edit) {
				edits.push(edit);
			}
		}

		return edits;
	}
}
