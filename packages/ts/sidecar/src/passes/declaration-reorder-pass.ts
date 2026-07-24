import type { AstReader } from '#sidecar/syntax/ast-reader';
import { isErr } from '#sidecar/kernel/result';
import { Node } from '#sidecar/syntax/node-schema';
import type { SourceParser } from '#sidecar/syntax/source-parser';
import type { Edit } from '#sidecar/syntax/edits';
import type { FormattingPass } from '#sidecar/passes/pass';
import type { SourceDocument } from '#sidecar/syntax/source-document';

/** Reorders declarations only where the transformation is side-effect safe. */
export class DeclarationReorderPass implements FormattingPass {
	/** The pass identity used for reporting. */
	readonly name = 'declaration-reorder';

	readonly #parser: SourceParser;
	readonly #ast: AstReader;

	/**
	 * @param dependencies - The syntax services consumed by the pass.
	 * @param dependencies.parser - Parses source into a trustworthy tree.
	 * @param dependencies.ast - Traverses and reads validated node fields.
	 */
	constructor(dependencies: { parser: SourceParser; ast: AstReader }) {
		this.#parser = dependencies.parser;
		this.#ast = dependencies.ast;
	}

	/**
	 * Compute declaration-ordering edits.
	 *
	 * @param document - The document to inspect.
	 * @returns Safe declaration-ordering edits, or none for invalid source.
	 */
	computeEdits(document: SourceDocument): Edit[] {
		const parsed = this.#parser.parse(document.virtualName, document.text);

		if (isErr(parsed)) {
			return [];
		}

		const lists = this.#ast.collectStatementLists(parsed.value.program);
		const edits: Edit[] = [];

		for (const list of lists) {
			const importGroups = this.#splitGroups(list, (node) => {
				return node.type === 'ImportDeclaration';
			});

			const constGroups = this.#splitGroups(list, (node) => {
				return this.#ast.isConstDeclaration(node);
			});

			for (const group of importGroups) {
				const edit = this.#groupEdit(document, group, true);

				if (edit) {
					edits.push(edit);
				}
			}

			for (const group of constGroups) {
				const edit = this.#groupEdit(document, group, this.#canReorderConstGroup(document, group));

				if (edit) {
					edits.push(edit);
				}
			}
		}

		return edits;
	}

	#isMultiline(document: SourceDocument, node: Node): boolean {
		const start = this.#ast.getStart(node);
		const end = this.#ast.getEnd(node);

		return start >= 0 && end >= 0 && document.slice(start, end).includes('\n');
	}

	#nodeSource(document: SourceDocument, node: Node): string {
		const start = this.#ast.getStart(node);
		const end = this.#ast.getEnd(node);

		return `${document.lineIndent(start)}${document.slice(start, end)}`;
	}

	#isSideEffectSafeExpression(node: Node | undefined): boolean {
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

						return element instanceof Node && this.#isSideEffectSafeExpression(element);
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
							return this.#isSideEffectSafeExpression(this.#ast.childNode(property, 'argument'));
						}

						if (property.type !== 'ObjectProperty' && property.type !== 'Property') {
							return false;
						}

						const computed = Boolean(property.computed);
						const key = this.#ast.childNode(property, 'key');
						const value = this.#ast.childNode(property, 'value');

						return (!computed || this.#isSideEffectSafeExpression(key)) && this.#isSideEffectSafeExpression(value);
					})
				);
			}

			case 'TemplateLiteral': {
				const expressions = node.expressions;

				return (
					Array.isArray(expressions) &&
					expressions.every((expression) => {
						return expression instanceof Node && this.#isSideEffectSafeExpression(expression);
					})
				);
			}

			default:
				return false;
		}
	}

	#isSafeConstDeclaration(node: Node): boolean {
		if (!this.#ast.isConstDeclaration(node)) {
			return false;
		}

		return (
			Array.isArray(node.declarations) &&
			this.#ast.childNodes(node, 'declarations').every((declaration) => {
				const id = this.#ast.childNode(declaration, 'id');

				return id?.type === 'Identifier' && this.#isSideEffectSafeExpression(this.#ast.childNode(declaration, 'init'));
			})
		);
	}

	#declaredNames(nodes: Node[]): Set<string> {
		const names = new Set<string>();

		for (const node of nodes) {
			for (const declaration of this.#ast.childNodes(node, 'declarations')) {
				const id = this.#ast.childNode(declaration, 'id');
				const name = id ? this.#ast.nodeName(id) : undefined;

				if (id?.type === 'Identifier' && name !== undefined) {
					names.add(name);
				}
			}
		}

		return names;
	}

	#usesAnyIdentifier(node: Node, names: Set<string>): boolean {
		let found = false;

		this.#ast.visit(node, (child) => {
			if (found || child.type !== 'Identifier') {
				return;
			}

			const name = this.#ast.nodeName(child);

			if (name !== undefined && names.has(name)) {
				found = true;
			}
		});

		return found;
	}

	#canReorderConstGroup(document: SourceDocument, group: Node[]): boolean {
		if (
			!group.every((node) => {
				return this.#isSafeConstDeclaration(node);
			})
		) {
			return false;
		}

		for (let i = 0; i < group.length; i++) {
			const node = group[i];

			if (!node || !this.#isMultiline(document, node)) {
				continue;
			}

			const names = this.#declaredNames([node]);

			if (
				group.slice(i + 1).some((node) => {
					return !this.#isMultiline(document, node) && this.#usesAnyIdentifier(node, names);
				})
			) {
				return false;
			}
		}

		return true;
	}

	#splitGroups(list: Node[], predicate: (node: Node) => boolean): Node[][] {
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

	#groupEdit(document: SourceDocument, group: Node[], canReorder: boolean): Edit | null {
		const singleLine = group.filter((node) => {
			return !this.#isMultiline(document, node);
		});
		const multiline = group.filter((node) => {
			return this.#isMultiline(document, node);
		});

		if (singleLine.length === 0 || multiline.length === 0) {
			return null;
		}

		const desired = canReorder ? [...singleLine, ...multiline] : group;

		const replacement = desired
			.map((node, index) => {
				const previous = desired[index - 1];
				const separator = previous && (this.#isMultiline(document, previous) || this.#isMultiline(document, node)) ? '\n\n' : index > 0 ? '\n' : '';

				return `${separator}${this.#nodeSource(document, node)}`;
			})
			.join('');

		const first = group[0];
		const last = group.at(-1);

		if (!first || !last) {
			return null;
		}

		const firstStart = this.#ast.getStart(first);
		const lastEnd = this.#ast.getEnd(last);

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
}
