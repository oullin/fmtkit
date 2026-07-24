import { AstReader } from '#sidecar/syntax/ast-reader';
import { Node } from '#sidecar/syntax/node-schema';
import { isErr } from '#sidecar/kernel/result';
import { SourceDocument } from '#sidecar/syntax/source-document';
import { SourceParser } from '#sidecar/syntax/source-parser';
import type { Edit } from '#sidecar/syntax/edits';

/** Reorders declarations only where the transformation is side-effect safe. */
export class DeclarationReorder {
	static readonly #ast = new AstReader();

	static readonly #parser = new SourceParser();

	static #isMultiline(document: SourceDocument, node: Node): boolean {
		const start = DeclarationReorder.#ast.getStart(node);
		const end = DeclarationReorder.#ast.getEnd(node);

		return start >= 0 && end >= 0 && document.slice(start, end).includes('\n');
	}

	static #nodeSource(document: SourceDocument, node: Node): string {
		const start = DeclarationReorder.#ast.getStart(node);
		const end = DeclarationReorder.#ast.getEnd(node);

		return `${document.lineIndent(start)}${document.slice(start, end)}`;
	}

	static #isSideEffectSafeExpression(node: Node | undefined): boolean {
		if (!node) {
			return true;
		}

		switch (node.type) {
			case 'ArrowFunctionExpression':

			case 'FunctionExpression':

			case 'Identifier':

			case 'Literal':
				return true;

			case 'ArrayExpression': {
				const elements = node.elements;

				return (
					Array.isArray(elements) &&
					elements.every((element) => {
						if (element === null) {
							return true;
						}

						return element instanceof Node && DeclarationReorder.#isSideEffectSafeExpression(element);
					})
				);
			}

			case 'ObjectExpression': {
				const properties = node.properties;

				return (
					Array.isArray(properties) &&
					properties.every((property) => {
						if (!(property instanceof Node)) {
							return false;
						}

						if (property.type === 'SpreadElement') {
							return DeclarationReorder.#isSideEffectSafeExpression(DeclarationReorder.#ast.childNode(property, 'argument'));
						}

						if (property.type !== 'ObjectProperty' && property.type !== 'Property') {
							return false;
						}

						const computed = Boolean(property.computed);
						const key = DeclarationReorder.#ast.childNode(property, 'key');
						const value = DeclarationReorder.#ast.childNode(property, 'value');

						return (!computed || DeclarationReorder.#isSideEffectSafeExpression(key)) && DeclarationReorder.#isSideEffectSafeExpression(value);
					})
				);
			}

			case 'TemplateLiteral': {
				const expressions = node.expressions;

				return (
					Array.isArray(expressions) &&
					expressions.every((expression) => {
						return expression instanceof Node && DeclarationReorder.#isSideEffectSafeExpression(expression);
					})
				);
			}

			default:
				return false;
		}
	}

	static #isSafeConstDeclaration(node: Node): boolean {
		if (!DeclarationReorder.#ast.isConstDeclaration(node)) {
			return false;
		}

		return (
			Array.isArray(node.declarations) &&
			DeclarationReorder.#ast.childNodes(node, 'declarations').every((declaration) => {
				const id = DeclarationReorder.#ast.childNode(declaration, 'id');

				return id?.type === 'Identifier' && DeclarationReorder.#isSideEffectSafeExpression(DeclarationReorder.#ast.childNode(declaration, 'init'));
			})
		);
	}

	static #declaredNames(nodes: Node[]): Set<string> {
		const names = new Set<string>();

		for (const node of nodes) {
			for (const declaration of DeclarationReorder.#ast.childNodes(node, 'declarations')) {
				const id = DeclarationReorder.#ast.childNode(declaration, 'id');
				const name = id ? DeclarationReorder.#ast.nodeName(id) : undefined;

				if (id?.type === 'Identifier' && name !== undefined) {
					names.add(name);
				}
			}
		}

		return names;
	}

	static #usesAnyIdentifier(node: Node, names: Set<string>): boolean {
		let found = false;

		DeclarationReorder.#ast.visit(node, (child) => {
			if (found || child.type !== 'Identifier') {
				return;
			}

			const name = DeclarationReorder.#ast.nodeName(child);

			if (name !== undefined && names.has(name)) {
				found = true;
			}
		});

		return found;
	}

	static #canReorderConstGroup(document: SourceDocument, group: Node[]): boolean {
		if (
			!group.every((node) => {
				return DeclarationReorder.#isSafeConstDeclaration(node);
			})
		) {
			return false;
		}

		for (let i = 0; i < group.length; i++) {
			const node = group[i];

			if (!node || !DeclarationReorder.#isMultiline(document, node)) {
				continue;
			}

			const names = DeclarationReorder.#declaredNames([node]);

			if (
				group.slice(i + 1).some((node) => {
					return !DeclarationReorder.#isMultiline(document, node) && DeclarationReorder.#usesAnyIdentifier(node, names);
				})
			) {
				return false;
			}
		}

		return true;
	}

	static #splitGroups(list: Node[], predicate: (node: Node) => boolean): Node[][] {
		const groups: Node[][] = [];

		let current: Node[] = [];

		for (const node of list) {
			if (!predicate(node)) {
				if (current.length > 1) {
					groups.push(current);
				}

				current = [];
				continue;
			}

			current.push(node);
		}

		if (current.length > 1) {
			groups.push(current);
		}

		return groups;
	}

	static #groupEdit(document: SourceDocument, group: Node[], canReorder: boolean): Edit | null {
		const singleLine = group.filter((node) => {
			return !DeclarationReorder.#isMultiline(document, node);
		});
		const multiline = group.filter((node) => {
			return DeclarationReorder.#isMultiline(document, node);
		});

		if (singleLine.length === 0 || multiline.length === 0) {
			return null;
		}

		const desired = canReorder ? [...singleLine, ...multiline] : group;

		const replacement = desired
			.map((node, index) => {
				const previous = desired[index - 1];
				const separator = previous && (DeclarationReorder.#isMultiline(document, previous) || DeclarationReorder.#isMultiline(document, node)) ? '\n\n' : index > 0 ? '\n' : '';

				return `${separator}${DeclarationReorder.#nodeSource(document, node)}`;
			})
			.join('');

		const first = group[0];
		const last = group.at(-1);

		if (!first || !last) {
			return null;
		}

		const firstStart = DeclarationReorder.#ast.getStart(first);
		const lastEnd = DeclarationReorder.#ast.getEnd(last);

		if (firstStart < 0 || lastEnd < 0) {
			return null;
		}

		const start = document.lineStart(firstStart);
		const current = document.slice(start, lastEnd);

		if (current === replacement) {
			return null;
		}

		const alreadyOrdered = desired.every((node, index) => {
			return node === group[index];
		});

		if (!canReorder && !alreadyOrdered) {
			return null;
		}

		return {
			start,
			end: lastEnd,
			replacement,
		};
	}
	/**
	 * Compute declaration-ordering edits.
	 *
	 * @param content - The source text to inspect.
	 * @param virtualName - The filename used to parse the source.
	 * @returns Safe declaration-ordering edits, or none for invalid source.
	 */
	static computeEdits(content: string, virtualName: string): Edit[] {
		const parsed = DeclarationReorder.#parser.parse(virtualName, content);

		if (isErr(parsed)) {
			return [];
		}

		const document = SourceDocument.of(virtualName, content);
		const lists = DeclarationReorder.#ast.collectStatementLists(parsed.value.program);
		const edits: Edit[] = [];

		for (const list of lists) {
			const importGroups = DeclarationReorder.#splitGroups(list, (node) => {
				return node.type === 'ImportDeclaration';
			});

			const constGroups = DeclarationReorder.#splitGroups(list, (node) => {
				return DeclarationReorder.#ast.isConstDeclaration(node);
			});

			for (const group of importGroups) {
				const edit = DeclarationReorder.#groupEdit(document, group, true);

				if (edit) {
					edits.push(edit);
				}
			}

			for (const group of constGroups) {
				const edit = DeclarationReorder.#groupEdit(document, group, DeclarationReorder.#canReorderConstGroup(document, group));

				if (edit) {
					edits.push(edit);
				}
			}
		}

		return edits;
	}
}
