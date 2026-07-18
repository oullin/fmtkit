import { childNode, childNodes, declarationKind, isNode, nodeName } from '#sidecar/ast';
import { isConstDeclaration } from '#sidecar/pass-utils';
import type { Node } from '#sidecar/types';

const BLOCK_HAVING_STATEMENTS = new Set(['IfStatement', 'ForStatement', 'ForInStatement', 'ForOfStatement', 'WhileStatement', 'DoWhileStatement', 'SwitchStatement', 'TryStatement']);
const LOOP_STATEMENTS = new Set(['ForStatement', 'ForInStatement', 'ForOfStatement', 'WhileStatement', 'DoWhileStatement']);
const TS_TYPE_DECLARATION_TYPES = new Set(['TSTypeAliasDeclaration', 'TSInterfaceDeclaration', 'TSEnumDeclaration', 'TSModuleDeclaration']);
const CLASS_METHOD_TYPES = new Set(['MethodDefinition', 'TSAbstractMethodDefinition']);
const CLASS_PROPERTY_TYPES = new Set(['PropertyDefinition', 'TSAbstractPropertyDefinition', 'AccessorProperty', 'TSIndexSignature', 'StaticBlock']);
const BLANK_LINE_ABOVE_TYPES = new Set(['SwitchStatement', 'SwitchCase', 'FunctionDeclaration', 'ClassDeclaration', 'TSEnumDeclaration', 'TSModuleDeclaration']);

const STRUCTURED_PREVIOUS_STATEMENTS = new Set([
	'ClassDeclaration',
	'DoWhileStatement',
	'ForInStatement',
	'ForOfStatement',
	'ForStatement',
	'FunctionDeclaration',
	'IfStatement',
	'SwitchStatement',
	'TryStatement',
	'WhileStatement',
]);

const VUE_PRIMITIVE_CALLS = new Set([
	'computed',
	'nextTick',
	'onActivated',
	'onBeforeMount',
	'onBeforeUnmount',
	'onBeforeUpdate',
	'onDeactivated',
	'onErrorCaptured',
	'onMounted',
	'onRenderTracked',
	'onRenderTriggered',
	'onServerPrefetch',
	'onUnmounted',
	'onUpdated',
	'reactive',
	'readonly',
	'ref',
	'shallowReactive',
	'shallowRef',
	'watch',
	'watchEffect',
]);

function isExportWithDeclaration(n: Node): boolean {
	if (n.type !== 'ExportNamedDeclaration' && n.type !== 'ExportDefaultDeclaration') {
		return false;
	}

	return Boolean(n.declaration);
}

function isBlankLineAboveType(next: Node): boolean {
	return BLANK_LINE_ABOVE_TYPES.has(next.type);
}

function isIdentifierNamed(node: Node | undefined, names: Set<string>): boolean {
	if (node?.type !== 'Identifier') {
		return false;
	}

	const name = nodeName(node);

	return typeof name === 'string' && names.has(name);
}

function isVuePrimitiveCall(node: Node | undefined): boolean {
	if (!node || node.type !== 'CallExpression') {
		return false;
	}

	return isIdentifierNamed(
		childNode(node, 'callee'),
		VUE_PRIMITIVE_CALLS,
	);
}

function isVuePrimitiveStatement(node: Node): boolean {
	if (node.type === 'ExpressionStatement') {
		return isVuePrimitiveCall(
			childNode(node, 'expression'),
		);
	}

	if (node.type !== 'VariableDeclaration' || declarationKind(node) !== 'const') {
		return false;
	}

	return childNodes(node, 'declarations').some((declaration) => {
		return isVuePrimitiveCall(
			childNode(declaration, 'init'),
		);
	});
}

function needsBlankLineAbove(next: Node): boolean {
	if (next.type === 'ReturnStatement') {
		return true;
	}

	if (isVuePrimitiveStatement(next)) {
		return true;
	}

	if (isBlankLineAboveType(next)) {
		return true;
	}

	return isExportWithDeclaration(next);
}

function isTypeDeclarationAbove(prev: Node): boolean {
	if (TS_TYPE_DECLARATION_TYPES.has(prev.type)) {
		return true;
	}

	if (prev.type === 'ExportNamedDeclaration') {
		const declType = childNode(prev, 'declaration')?.type;

		return declType ? TS_TYPE_DECLARATION_TYPES.has(declType) : false;
	}

	return false;
}

function isLoopStatement(node: Node): boolean {
	return LOOP_STATEMENTS.has(node.type);
}

function isStructuredPreviousStatement(prev: Node): boolean {
	if (STRUCTURED_PREVIOUS_STATEMENTS.has(prev.type)) {
		return true;
	}

	if (prev.type === 'ExportNamedDeclaration' || prev.type === 'ExportDefaultDeclaration') {
		const declType = childNode(prev, 'declaration')?.type;

		return Boolean(declType && STRUCTURED_PREVIOUS_STATEMENTS.has(declType));
	}

	return false;
}

function isClassMethodPair(prev: Node, next: Node): boolean {
	return CLASS_METHOD_TYPES.has(prev.type) && CLASS_METHOD_TYPES.has(next.type);
}

function isPropertyToMethodTransition(prev: Node, next: Node): boolean {
	return CLASS_PROPERTY_TYPES.has(prev.type) && CLASS_METHOD_TYPES.has(next.type);
}

function isLetDeclaration(node: Node): boolean {
	return node.type === 'VariableDeclaration' && declarationKind(node) === 'let';
}

function containsAwait(node: Node): boolean {
	if (node.type === 'AwaitExpression') {
		return true;
	}

	if (node.type === 'FunctionDeclaration' || node.type === 'FunctionExpression' || node.type === 'ArrowFunctionExpression') {
		return false;
	}

	for (const value of Object.values(node)) {
		if (!value || typeof value !== 'object') {
			continue;
		}

		if (Array.isArray(value)) {
			if (
				value.some((child) => {
					return isNode(child) && containsAwait(child);
				})
			) {
				return true;
			}
		} else if (isNode(value) && containsAwait(value)) {
			return true;
		}
	}

	return false;
}

/** Encapsulates the formatter's statement and class-member layout rules. */
export class Rules {
	/**
	 * Decide whether two adjacent statements require a blank line.
	 *
	 * @param prev - The previous statement.
	 * @param next - The following statement.
	 * @returns `true` when the pair must be separated by a blank line.
	 */
	static needsBlankLine(prev: Node, next: Node): boolean {
		if (containsAwait(prev) || containsAwait(next)) {
			return true;
		}

		if (needsBlankLineAbove(next)) {
			return true;
		}

		if (isLoopStatement(next)) {
			return !isStructuredPreviousStatement(prev);
		}

		if (isClassMethodPair(prev, next)) {
			return true;
		}

		if (isPropertyToMethodTransition(prev, next)) {
			return true;
		}

		if (isTypeDeclarationAbove(prev)) {
			return true;
		}

		if (prev.type === 'ImportDeclaration' && next.type !== 'ImportDeclaration') {
			return true;
		}

		if (isConstDeclaration(prev) !== isConstDeclaration(next)) {
			return true;
		}

		if (isLetDeclaration(prev) !== isLetDeclaration(next)) {
			return true;
		}

		if (prev.type === 'VariableDeclaration' && next.type !== 'VariableDeclaration') {
			return true;
		}

		if (BLOCK_HAVING_STATEMENTS.has(prev.type)) {
			return true;
		}

		return false;
	}

	/**
	 * Classify a class member for stable ordering.
	 *
	 * @param node - The class member to classify.
	 * @returns Its property, constructor, or method group.
	 */
	static classifyMember(node: Node): 'property' | 'constructor' | 'method' {
		if (CLASS_PROPERTY_TYPES.has(node.type)) {
			return 'property';
		}

		if (node.type === 'MethodDefinition' && declarationKind(node) === 'constructor') {
			return 'constructor';
		}

		return 'method';
	}
}
