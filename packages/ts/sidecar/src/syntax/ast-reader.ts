import { z } from 'zod';
import { Node } from '#sidecar/syntax/node-schema';
import type { AstValue } from '#sidecar/syntax/node-schema';

/** The opening and closing argument-parenthesis offsets of a call. */
export type CallParens = {
	/** The opening parenthesis offset. */
	readonly open: number;

	/** The closing parenthesis offset. */
	readonly close: number;
};

const STATEMENT_LIST_KEYS: Record<string, 'body' | 'consequent'> = {
	Program: 'body',
	BlockStatement: 'body',
	StaticBlock: 'body',
	SwitchCase: 'consequent',
	ClassBody: 'body',
};

const stringSchema = z.string();

/** Traverses boundary-admitted AST nodes and validates narrow scalar reads. */
export class AstReader {
	/**
	 * Read a child node off a parent property.
	 *
	 * @param node - The parent node.
	 * @param key - The property to read.
	 * @returns The child when the property holds a node, `undefined` otherwise.
	 */
	childNode(node: Node, key: string): Node | undefined {
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
	childNodes(node: Node, key: string): Node[] {
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
	nodeName(node: Node): string | undefined {
		return this.#stringValue(node.name);
	}

	/**
	 * Lazily validate and read a trusted node's optional `kind` property.
	 *
	 * @param node - The node whose declaration kind to read.
	 * @returns The validated kind, or `undefined` when it is absent or invalid.
	 */
	declarationKind(node: Node): string | undefined {
		return this.#stringValue(node.kind);
	}

	/**
	 * Lazily validate and read a trusted literal or comment's string value.
	 *
	 * @param node - The literal or comment node whose value to read.
	 * @returns The validated value, or `undefined` when it is absent or invalid.
	 */
	stringValue(node: Node): string | undefined {
		return this.#stringValue(node.value);
	}

	/**
	 * Report whether an AST node is a `const` variable declaration.
	 *
	 * @param node - The node to inspect.
	 * @returns `true` when the node declares one or more constants.
	 */
	isConstDeclaration(node: Node): boolean {
		return node.type === 'VariableDeclaration' && this.declarationKind(node) === 'const';
	}

	/**
	 * Read a node's source start with its range as a fallback.
	 *
	 * @param node - The node whose start offset to read.
	 * @returns The source start, or `-1` when no position is available.
	 */
	getStart(node: Node): number {
		return node.start ?? node.range?.[0] ?? -1;
	}

	/**
	 * Read a node's source end with its range as a fallback.
	 *
	 * @param node - The node whose end offset to read.
	 * @returns The source end, or `-1` when no position is available.
	 */
	getEnd(node: Node): number {
		return node.end ?? node.range?.[1] ?? -1;
	}

	/**
	 * Visit every validated node in depth-first order.
	 *
	 * @param node - The root node to traverse.
	 * @param visitor - The operation invoked once for each node.
	 * @returns Nothing.
	 */
	visit(node: Node, visitor: (node: Node) => void): void {
		visitor(node);

		for (const value of Object.values(node)) {
			if (Array.isArray(value)) {
				for (const child of value) {
					if (child instanceof Node) {
						this.visit(child, visitor);
					}
				}
			} else if (value instanceof Node) {
				this.visit(value, visitor);
			}
		}
	}

	/**
	 * Collect statement arrays that participate in blank-line rules.
	 *
	 * @param program - The program root to traverse.
	 * @returns Statement lists in depth-first traversal order.
	 */
	collectStatementLists(program: Node): Node[][] {
		const lists: Node[][] = [];

		this.visit(program, (node) => {
			const key = STATEMENT_LIST_KEYS[node.type];

			if (key && Array.isArray(node[key])) {
				lists.push(this.childNodes(node, key));
			}

			if (node.type === 'SwitchStatement' && Array.isArray(node.cases)) {
				lists.push(this.childNodes(node, 'cases'));
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
	collectClassBodies(program: Node): Node[] {
		const bodies: Node[] = [];

		this.visit(program, (node) => {
			if (node.type === 'ClassBody') {
				bodies.push(node);
			}
		});

		return bodies;
	}

	/**
	 * Slice the source range occupied by an AST node.
	 *
	 * @param source - The complete source text.
	 * @param node - The node whose source to read.
	 * @returns The node's source text.
	 */
	sourceOf(source: string, node: Node): string {
		return source.slice(this.getStart(node), this.getEnd(node));
	}

	/**
	 * Locate the argument parentheses of a call with a caller-unwrapped callee.
	 *
	 * @param source - The complete source text.
	 * @param call - The call expression node.
	 * @param callee - The callee after the caller's own unwrapping rules.
	 * @returns The parenthesis offsets, or `null` when they cannot be located.
	 */
	callParens(source: string, call: Node, callee: Node | undefined): CallParens | null {
		const calleeEnd = callee ? this.getEnd(callee) : -1;
		const callEnd = this.getEnd(call);

		if (calleeEnd < 0 || callEnd < 0) {
			return null;
		}

		const open = source.indexOf('(', calleeEnd);

		if (open < 0 || open >= callEnd) {
			return null;
		}

		const close = callEnd - 1;

		if (source[close] !== ')') {
			return null;
		}

		return { open, close };
	}

	/**
	 * Unwrap an ESTree chain expression when present.
	 *
	 * @param node - The possible chain expression.
	 * @returns Its expression child, or the original node.
	 */
	unwrapChainExpression(node: Node | undefined): Node | undefined {
		if (node?.type === 'ChainExpression') {
			return this.childNode(node, 'expression');
		}

		return node;
	}

	#stringValue(value: AstValue): string | undefined {
		const parsed = stringSchema.safeParse(value);

		return parsed.success ? parsed.data : undefined;
	}
}
