import type { AstReader } from '#sidecar/syntax/ast-reader';
import { isErr } from '#sidecar/kernel/result';
import type { ParsedSourceDto } from '#sidecar/syntax/node-schema';
import type { SourceParser } from '#sidecar/syntax/source-parser';
import type { Edit } from '#sidecar/syntax/edits';
import type { FormattingPass } from '#sidecar/passes/pass';
import type { Node } from '#sidecar/syntax/node-schema';
import type { SourceDocument } from '#sidecar/syntax/source-document';

type ChainLink = {
	end: number;
	operator: '.' | '?.';
	start: number;
};

type FluentChain = {
	base: Node;
	links: Array<ChainLink>;
};

/** Splits fluent-call chains so each link starts on its own line. */
export class FluentChainPass implements FormattingPass {
	/** The pass identity used for reporting. */
	readonly name = 'fluent-chains';

	readonly #parser: SourceParser;
	readonly #ast: AstReader;

	/**
	 * @param dependencies - The syntax services consumed by the pass.
	 * @param dependencies.parser - Parses source into a trustworthy tree.
	 * @param dependencies.ast - Traverses and reads validated node fields.
	 */
	constructor(dependencies: { ast: AstReader; parser: SourceParser }) {
		this.#parser = dependencies.parser;
		this.#ast = dependencies.ast;
	}

	/**
	 * Compute edits that split fluent-chain links across lines.
	 *
	 * @param document - The document to inspect.
	 * @returns Fluent-chain edits, or none for invalid source.
	 */
	computeEdits(document: SourceDocument): Array<Edit> {
		const parsed = this.#parser.parse(document.virtualName, document.text);

		if (isErr(parsed)) {
			return [];
		}

		const edits = new Map<string, Edit>();
		const indentStep = document.indentUnit();

		this.#ast.visit(parsed.value.program, (node) => {
			if (node.type !== 'CallExpression') {
				return;
			}

			const chain = this.#collectFluentChain(document, node, parsed.value);

			if (!chain) {
				return;
			}

			const baseStart = this.#ast.getStart(chain.base);

			if (baseStart < 0) {
				return;
			}

			const indent = `${document.lineIndent(baseStart)}${indentStep}`;

			for (const link of chain.links) {
				const replacement = `\n${indent}${link.operator}`;

				if (document.slice(link.start, link.end) === replacement) {
					continue;
				}

				edits.set(`${link.start}:${link.end}`, {
					end: link.end,
					replacement,
					start: link.start,
				});
			}
		});

		return [...edits.values()].sort((a, b) => {
			return a.start - b.start;
		});
	}

	#memberCallLink(document: SourceDocument, member: Node, object: Node, parsed: ParsedSourceDto): ChainLink | null {
		if (member.computed) {
			return null;
		}

		const property = this.#ast.childNode(member, 'property');

		if (!property || (property.type !== 'Identifier' && property.type !== 'PrivateIdentifier')) {
			return null;
		}

		const objectEnd = this.#ast.getEnd(object);
		const propertyStart = this.#ast.getStart(property);

		if (objectEnd < 0 || propertyStart < 0 || propertyStart <= objectEnd) {
			return null;
		}

		if (parsed.hasCommentBetween(objectEnd, propertyStart)) {
			return null;
		}

		const operator = document.slice(objectEnd, propertyStart).replaceAll(/[ \t\r\n]/g, '');

		if (operator !== '.' && operator !== '?.') {
			return null;
		}

		return {
			end: propertyStart,
			operator,
			start: objectEnd,
		};
	}

	#collectFluentChain(document: SourceDocument, outer: Node, parsed: ParsedSourceDto): FluentChain | null {
		let call: Node = outer;

		const links: Array<ChainLink> = [];

		while (call.type === 'CallExpression') {
			const callee = this.#ast.unwrapChainExpression(this.#ast.childNode(call, 'callee'));

			if (callee?.type !== 'MemberExpression') {
				break;
			}

			const object = this.#ast.unwrapChainExpression(this.#ast.childNode(callee, 'object'));

			if (object?.type !== 'CallExpression') {
				break;
			}

			const link = this.#memberCallLink(document, callee, object, parsed);

			if (!link) {
				return null;
			}

			links.push(link);
			call = object;
		}

		if (links.length < 2) {
			return null;
		}

		return {
			base: call,
			links,
		};
	}
}
