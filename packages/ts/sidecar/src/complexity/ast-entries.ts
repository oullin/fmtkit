import { Node } from '#sidecar/syntax/node-schema';
import { z } from 'zod';

/** One AST child paired with the property it hangs off. */
export type AstEntry = {
	/** The parent property the child was read from. */
	readonly key: string;

	/** The child node. */
	readonly child: Node;
};

/** The three node types that own a parameter list and a body. */
export const FUNCTION_TYPES: ReadonlySet<string> = new Set(['ArrowFunctionExpression', 'FunctionDeclaration', 'FunctionExpression']);

const stringSchema = z.string();

/**
 * Report whether a node introduces its own function scope.
 *
 * Methods, constructors, and accessors are not their own types in ESTree: they
 * are `MethodDefinition` or `PropertyDefinition` wrappers around one of these
 * three, so scoring the three covers all six shapes the check enumerates.
 *
 * @param node - The node to classify.
 * @returns `true` when the node is function-like.
 */
export function isFunctionNode(node: Node): boolean {
	return FUNCTION_TYPES.has(node.type);
}

/**
 * Enumerate a node's AST children with the properties they came from.
 *
 * Structure below the parser boundary is trusted Oxc output, so children are
 * recognised structurally rather than re-validated; the property name comes
 * back with each child because the nesting rules are keyed by it.
 *
 * @param node - The parent node to enumerate.
 * @returns The child entries in the parser's own property order.
 */
export function childEntries(node: Node): AstEntry[] {
	const entries: AstEntry[] = [];

	for (const [key, value] of Object.entries(node)) {
		if (Array.isArray(value)) {
			for (const item of value) {
				if (item instanceof Node) {
					entries.push({ key, child: item });
				}
			}

			continue;
		}

		if (value instanceof Node) {
			entries.push({ key, child: value });
		}
	}

	return entries;
}

/**
 * Read one child node off a parent property.
 *
 * @param node - The parent node.
 * @param key - The property to read.
 * @returns The child when the property holds a node, `undefined` otherwise.
 */
export function childNode(node: Node, key: string): Node | undefined {
	const value = node[key];

	return value instanceof Node ? value : undefined;
}

/**
 * Lazily validate and read a string-valued property off a trusted node.
 *
 * @param node - The node to read from.
 * @param key - The property to read.
 * @returns The validated string, or `''` when the property is absent or not one.
 */
export function stringProperty(node: Node, key: string): string {
	const parsed = stringSchema.safeParse(node[key]);

	return parsed.success ? parsed.data : '';
}
