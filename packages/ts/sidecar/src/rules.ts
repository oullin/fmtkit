import { isConstDeclaration } from '#devx/pass-utils';
import type { Node } from '#devx/types';

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

	const name = (node as { name?: unknown }).name;

	return typeof name === 'string' && names.has(name);
}

function isVuePrimitiveCall(node: Node | undefined): boolean {
	if (!node || node.type !== 'CallExpression') {
		return false;
	}

	return isIdentifierNamed(node.callee as Node | undefined, VUE_PRIMITIVE_CALLS);
}

function isVuePrimitiveStatement(node: Node): boolean {
	if (node.type === 'ExpressionStatement') {
		return isVuePrimitiveCall(node.expression as Node | undefined);
	}

	if (node.type !== 'VariableDeclaration' || (node as { kind?: unknown }).kind !== 'const') {
		return false;
	}

	const declarations = node.declarations as Node[] | undefined;

	return (
		Array.isArray(declarations) &&
		declarations.some((declaration) => {
			return isVuePrimitiveCall(declaration.init as Node | undefined);
		})
	);
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
		const declType = (prev.declaration as Node | undefined)?.type;

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
		const declType = (prev.declaration as Node | undefined)?.type;

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
	return node.type === 'VariableDeclaration' && (node as { kind?: unknown }).kind === 'let';
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
					return child && typeof child === 'object' && typeof (child as Node).type === 'string' && containsAwait(child as Node);
				})
			) {
				return true;
			}
		} else if (typeof (value as Node).type === 'string' && containsAwait(value as Node)) {
			return true;
		}
	}

	return false;
}

export function needsBlankLine(prev: Node, next: Node): boolean {
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

export function classifyMember(node: Node): 'property' | 'constructor' | 'method' {
	if (CLASS_PROPERTY_TYPES.has(node.type)) {
		return 'property';
	}

	if (node.type === 'MethodDefinition' && (node as { kind?: string }).kind === 'constructor') {
		return 'constructor';
	}

	return 'method';
}
