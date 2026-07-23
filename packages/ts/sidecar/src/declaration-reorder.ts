import { Ast } from '#sidecar/syntax/ast';
import { Node } from '#sidecar/syntax/node-schema';
import { isErr } from '#sidecar/kernel/result';
import { SourceText } from '#sidecar/syntax/source-text';
import { Sources } from '#sidecar/syntax/sources';
import type { Edit } from '#sidecar/syntax/edits';

/** Reorders declarations only where the transformation is side-effect safe. */
export class DeclarationReorder {
	static #isMultiline(source: string, node: Node): boolean {
		const start = Ast.getStart(node);
		const end = Ast.getEnd(node);

		return start >= 0 && end >= 0 && source.slice(start, end).includes('\n');
	}

	static #nodeSource(source: string, node: Node): string {
		const start = Ast.getStart(node);
		const end = Ast.getEnd(node);

		return `${SourceText.lineIndent(source, start)}${source.slice(start, end)}`;
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
							return DeclarationReorder.#isSideEffectSafeExpression(Ast.childNode(property, 'argument'));
						}

						if (property.type !== 'ObjectProperty' && property.type !== 'Property') {
							return false;
						}

						const computed = Boolean(property.computed);
						const key = Ast.childNode(property, 'key');
						const value = Ast.childNode(property, 'value');

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
		if (!Ast.isConstDeclaration(node)) {
			return false;
		}

		return (
			Array.isArray(node.declarations) &&
			Ast.childNodes(node, 'declarations').every((declaration) => {
				const id = Ast.childNode(declaration, 'id');

				return id?.type === 'Identifier' && DeclarationReorder.#isSideEffectSafeExpression(Ast.childNode(declaration, 'init'));
			})
		);
	}

	static #declaredNames(nodes: Node[]): Set<string> {
		const names = new Set<string>();

		for (const node of nodes) {
			for (const declaration of Ast.childNodes(node, 'declarations')) {
				const id = Ast.childNode(declaration, 'id');
				const name = id ? Ast.nodeName(id) : undefined;

				if (id?.type === 'Identifier' && name !== undefined) {
					names.add(name);
				}
			}
		}

		return names;
	}

	static #usesAnyIdentifier(node: Node, names: Set<string>): boolean {
		let found = false;

		Ast.visit(node, (child) => {
			if (found || child.type !== 'Identifier') {
				return;
			}

			const name = Ast.nodeName(child);

			if (name !== undefined && names.has(name)) {
				found = true;
			}
		});

		return found;
	}

	static #canReorderConstGroup(source: string, group: Node[]): boolean {
		if (
			!group.every((node) => {
				return DeclarationReorder.#isSafeConstDeclaration(node);
			})
		) {
			return false;
		}

		for (let i = 0; i < group.length; i++) {
			const node = group[i];

			if (!node || !DeclarationReorder.#isMultiline(source, node)) {
				continue;
			}

			const names = DeclarationReorder.#declaredNames([node]);

			if (
				group.slice(i + 1).some((node) => {
					return !DeclarationReorder.#isMultiline(source, node) && DeclarationReorder.#usesAnyIdentifier(node, names);
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

	static #groupEdit(source: string, group: Node[], canReorder: boolean): Edit | null {
		const singleLine = group.filter((node) => {
			return !DeclarationReorder.#isMultiline(source, node);
		});
		const multiline = group.filter((node) => {
			return DeclarationReorder.#isMultiline(source, node);
		});

		if (singleLine.length === 0 || multiline.length === 0) {
			return null;
		}

		const desired = canReorder ? [...singleLine, ...multiline] : group;

		const replacement = desired
			.map((node, index) => {
				const previous = desired[index - 1];
				const separator = previous && (DeclarationReorder.#isMultiline(source, previous) || DeclarationReorder.#isMultiline(source, node)) ? '\n\n' : index > 0 ? '\n' : '';

				return `${separator}${DeclarationReorder.#nodeSource(source, node)}`;
			})
			.join('');

		const first = group[0];
		const last = group.at(-1);

		if (!first || !last) {
			return null;
		}

		const firstStart = Ast.getStart(first);
		const lastEnd = Ast.getEnd(last);

		if (firstStart < 0 || lastEnd < 0) {
			return null;
		}

		const start = SourceText.lineStart(source, firstStart);
		const current = source.slice(start, lastEnd);

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
		const parsed = Sources.parse(virtualName, content);

		if (isErr(parsed)) {
			return [];
		}

		const lists = Ast.collectStatementLists(parsed.value.program);
		const edits: Edit[] = [];

		for (const list of lists) {
			const importGroups = DeclarationReorder.#splitGroups(list, (node) => {
				return node.type === 'ImportDeclaration';
			});

			const constGroups = DeclarationReorder.#splitGroups(list, Ast.isConstDeclaration);

			for (const group of importGroups) {
				const edit = DeclarationReorder.#groupEdit(content, group, true);

				if (edit) {
					edits.push(edit);
				}
			}

			for (const group of constGroups) {
				const edit = DeclarationReorder.#groupEdit(content, group, DeclarationReorder.#canReorderConstGroup(content, group));

				if (edit) {
					edits.push(edit);
				}
			}
		}

		return edits;
	}
}
