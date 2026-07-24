import type { AstReader } from '#sidecar/syntax/ast-reader';
import type { ClassMemberPolicy } from '#sidecar/passes/policies/class-member-policy';
import { isErr } from '#sidecar/kernel/result';
import type { SourceParser } from '#sidecar/syntax/source-parser';
import type { Edit } from '#sidecar/syntax/edits';
import type { FormattingPass } from '#sidecar/passes/pass';
import type { Node } from '#sidecar/syntax/node-schema';
import type { SourceDocument } from '#sidecar/syntax/source-document';

/** Reorders class members into the formatter's stable class shape. */
export class ClassReorderPass implements FormattingPass {
	/** The pass identity used for reporting. */
	readonly name = 'class-reorder';

	readonly #parser: SourceParser;
	readonly #ast: AstReader;
	readonly #members: ClassMemberPolicy;

	/**
	 * @param dependencies - The syntax services and policies consumed by the pass.
	 * @param dependencies.parser - Parses source into a trustworthy tree.
	 * @param dependencies.ast - Traverses and reads validated node fields.
	 * @param dependencies.members - Classifies members into their ordering group.
	 */
	constructor(dependencies: { parser: SourceParser; ast: AstReader; members: ClassMemberPolicy }) {
		this.#parser = dependencies.parser;
		this.#ast = dependencies.ast;
		this.#members = dependencies.members;
	}

	/**
	 * Compute class-member ordering edits.
	 *
	 * @param document - The document to inspect.
	 * @returns Class-member ordering edits, or none for invalid source.
	 */
	computeEdits(document: SourceDocument): Edit[] {
		const parsed = this.#parser.parse(document.virtualName, document.text);

		if (isErr(parsed)) {
			return [];
		}

		const source = document.text;
		const edits: Edit[] = [];

		for (const body of this.#ast.collectClassBodies(parsed.value.program)) {
			const edit = this.#computeClassReorderEdit(source, body);

			if (edit) {
				edits.push(edit);
			}
		}

		return edits;
	}

	#computeClassReorderEdit(source: string, body: Node): Edit | null {
		const members = this.#ast.childNodes(body, 'body');

		if (members.length < 2) {
			return null;
		}

		const properties: Node[] = [];
		const constructors: Node[] = [];
		const methods: Node[] = [];

		for (const member of members) {
			const kind = this.#members.classify(member);

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

		const bodyStart = this.#ast.getStart(body);
		const bodyEnd = this.#ast.getEnd(body);

		if (bodyStart < 0 || bodyEnd < 0 || this.#hasCommentsAroundMembers(source, body, members)) {
			return null;
		}

		const firstMember = members[0];
		const lastOriginal = members.at(-1);

		if (!firstMember || !lastOriginal) {
			return null;
		}

		const prefix = source.slice(bodyStart + 1, this.#ast.getStart(firstMember));
		const indent = prefix.match(/\n([ \t]*)$/)?.[1];

		if (indent === undefined) {
			return null;
		}

		const memberSlices = desired.map((member) => {
			return source.slice(this.#ast.getStart(member), this.#ast.getEnd(member));
		});

		const closing = source.slice(this.#ast.getEnd(lastOriginal), bodyEnd - 1);

		return {
			start: bodyStart + 1,
			end: bodyEnd - 1,
			replacement: `\n${indent}${memberSlices.join(`\n${indent}`)}${closing}`,
		};
	}

	#hasCommentsAroundMembers(source: string, body: Node, members: Node[]): boolean {
		const first = members[0];
		const last = members.at(-1);

		if (!first || !last) {
			return false;
		}

		const bodyStart = this.#ast.getStart(body);
		const bodyEnd = this.#ast.getEnd(body);
		const firstStart = this.#ast.getStart(first);
		const lastEnd = this.#ast.getEnd(last);

		if (this.#containsComment(source.slice(bodyStart + 1, firstStart))) {
			return true;
		}

		for (let i = 0; i < members.length - 1; i++) {
			const current = members[i];
			const following = members[i + 1];

			if (current && following && this.#containsComment(source.slice(this.#ast.getEnd(current), this.#ast.getStart(following)))) {
				return true;
			}
		}

		return this.#containsComment(source.slice(lastEnd, bodyEnd - 1));
	}

	#containsComment(source: string): boolean {
		return /\/\/|\/\*/.test(source);
	}
}
