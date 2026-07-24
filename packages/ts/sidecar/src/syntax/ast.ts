import { z } from 'zod';
import { Node } from '#sidecar/syntax/node-schema';
import type { AstValue } from '#sidecar/syntax/node-schema';

const STATEMENT_LIST_KEYS: Record<string, 'body' | 'consequent'> = {
	Program: 'body',
	BlockStatement: 'body',
	StaticBlock: 'body',
	SwitchCase: 'consequent',
	ClassBody: 'body',
};

/** Traverses boundary-admitted AST nodes and validates narrow scalar reads. */
export class Ast {
	static readonly #stringSchema = z.string();

	/**
	 * Read a child node off a parent property.
	 *
	 * @param node - The parent node.
	 * @param key - The property to read.
	 * @returns The child when the property holds a node, `undefined` otherwise.
	 */
	static childNode(node: Node, key: string): Node | undefined {
		const value = node[key];

		return value instanceof Node ? value : undefined;
	}

	/**
	 * Read an array of child nodes off a parent property.
	 *
	 * @param node - The parent node.
	 * @param key - The property to read.
	 * @returns The property's node entries, or an empty array when it is not an array.
	 */
	static childNodes(node: Node, key: string): Node[] {
		const value = node[key];

		return Array.isArray(value)
			? value.filter((child): child is Node => {
					return child instanceof Node;
				})
			: [];
	}

	/**
	 * Lazily validate and read a trusted node's optional `name` property.
	 *
	 * @param node - The node whose name to read.
	 * @returns The validated name, or `undefined` when it is absent or invalid.
	 */
	static nodeName(node: Node): string | undefined {
		return Ast.#stringValue(node.name);
	}

	/**
	 * Lazily validate and read a trusted node's optional `kind` property.
	 *
	 * @param node - The node whose declaration kind to read.
	 * @returns The validated kind, or `undefined` when it is absent or invalid.
	 */
	static declarationKind(node: Node): string | undefined {
		return Ast.#stringValue(node.kind);
	}

	/**
	 * Lazily validate and read a trusted literal or comment's string value.
	 *
	 * @param node - The literal or comment node whose value to read.
	 * @returns The validated value, or `undefined` when it is absent or invalid.
	 */
	static stringValue(node: Node): string | undefined {
		return Ast.#stringValue(node.value);
	}

	/**
	 * Report whether an AST node is a `const` variable declaration.
	 *
	 * @param node - The node to inspect.
	 * @returns `true` when the node declares one or more constants.
	 */
	static isConstDeclaration(node: Node): boolean {
		return node.type === 'VariableDeclaration' && Ast.declarationKind(node) === 'const';
	}

	/**
	 * Read a node's source start with its range as a fallback.
	 *
	 * @param node - The node whose start offset to read.
	 * @returns The source start, or `-1` when no position is available.
	 */
	static getStart(node: Node): number {
		return node.start ?? node.range?.[0] ?? -1;
	}

	/**
	 * Read a node's source end with its range as a fallback.
	 *
	 * @param node - The node whose end offset to read.
	 * @returns The source end, or `-1` when no position is available.
	 */
	static getEnd(node: Node): number {
		return node.end ?? node.range?.[1] ?? -1;
	}

	/**
	 * Visit every validated node in depth-first order.
	 *
	 * @param node - The root node to traverse.
	 * @param visitor - The operation invoked once for each node.
	 * @returns Nothing.
	 */
	static visit(node: Node, visitor: (node: Node) => void): void {
		visitor(node);

		for (const value of Object.values(node)) {
			if (Array.isArray(value)) {
				for (const child of value) {
					if (child instanceof Node) {
						Ast.visit(child, visitor);
					}
				}
			} else if (value instanceof Node) {
				Ast.visit(value, visitor);
			}
		}
	}

	/**
	 * Collect statement arrays that participate in blank-line rules.
	 *
	 * @param program - The program root to traverse.
	 * @returns Statement lists in depth-first traversal order.
	 */
	static collectStatementLists(program: Node): Node[][] {
		const lists: Node[][] = [];

		Ast.visit(program, (node) => {
			const key = STATEMENT_LIST_KEYS[node.type];

			if (key && Array.isArray(node[key])) {
				lists.push(Ast.childNodes(node, key));
			}

			if (node.type === 'SwitchStatement' && Array.isArray(node.cases)) {
				lists.push(Ast.childNodes(node, 'cases'));
			}
		});

		return lists;
	}

	/**
	 * Collect every class body below a parsed program.
	 *
	 * @param program - The program root to traverse.
	 * @returns Class bodies in depth-first traversal order.
	 */
	static collectClassBodies(program: Node): Node[] {
		const bodies: Node[] = [];

		Ast.visit(program, (node) => {
			if (node.type === 'ClassBody') {
				bodies.push(node);
			}
		});

		return bodies;
	}

	static #stringValue(value: AstValue): string | undefined {
		const parsed = Ast.#stringSchema.safeParse(value);

		return parsed.success ? parsed.data : undefined;
	}
}
