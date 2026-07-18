import type { Node } from '#sidecar/types';

const STATEMENT_LIST_KEYS: Record<string, 'body' | 'consequent'> = {
	Program: 'body',
	BlockStatement: 'body',
	StaticBlock: 'body',
	SwitchCase: 'consequent',
	ClassBody: 'body',
};

/**
 * Narrow an unknown value to an AST node.
 *
 * @param value - The value to inspect.
 * @returns `true` when `value` is an object carrying a string `type`.
 */
export function isNode(value: unknown): value is Node {
	if (typeof value !== 'object' || value === null) {
		return false;
	}

	if (!('type' in value)) {
		return false;
	}

	return typeof value.type === 'string';
}

/**
 * Read a child node off a parent property.
 *
 * @param node - The parent node.
 * @param key - The property to read.
 * @returns The child when the property holds a node, `undefined` otherwise.
 */
export function childNode(node: Node, key: string): Node | undefined {
	const value = node[key];

	return isNode(value) ? value : undefined;
}

/**
 * Read an array of child nodes off a parent property.
 *
 * @param node - The parent node.
 * @param key - The property to read.
 * @returns The property's node entries, or an empty array when it is not an array.
 */
export function childNodes(node: Node, key: string): Node[] {
	const value = node[key];

	return Array.isArray(value) ? value.filter(isNode) : [];
}

/**
 * Read a node's `name` property.
 *
 * @param node - The node to inspect.
 * @returns The name when it is a string, `undefined` otherwise.
 */
export function nodeName(node: Node): string | undefined {
	const name = node.name;

	return typeof name === 'string' ? name : undefined;
}

/**
 * Read a node's `kind` property.
 *
 * @param node - The node to inspect.
 * @returns The kind when it is a string, `undefined` otherwise.
 */
export function declarationKind(node: Node): string | undefined {
	const kind = node.kind;

	return typeof kind === 'string' ? kind : undefined;
}

export function getStart(n: Node): number {
	return typeof n.start === 'number' ? n.start : (n.range?.[0] ?? -1);
}

export function getEnd(n: Node): number {
	return typeof n.end === 'number' ? n.end : (n.range?.[1] ?? -1);
}

export function visit(node: Node, fn: (n: Node) => void): void {
	fn(node);

	for (const key of Object.keys(node)) {
		const value = node[key];

		if (Array.isArray(value)) {
			for (const child of value) {
				if (isNode(child)) {
					visit(child, fn);
				}
			}
		} else if (isNode(value)) {
			visit(value, fn);
		}
	}
}

export function collectStatementLists(program: Node): Node[][] {
	const lists: Node[][] = [];

	visit(program, (n) => {
		const key = STATEMENT_LIST_KEYS[n.type];

		if (key && Array.isArray(n[key])) {
			lists.push(childNodes(n, key));
		}

		if (n.type === 'SwitchStatement' && Array.isArray(n.cases)) {
			lists.push(childNodes(n, 'cases'));
		}
	});

	return lists;
}

export function collectClassBodies(program: Node): Node[] {
	const bodies: Node[] = [];

	visit(program, (n) => {
		if (n.type === 'ClassBody') {
			bodies.push(n);
		}
	});

	return bodies;
}
