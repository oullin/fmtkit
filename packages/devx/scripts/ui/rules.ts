import type { Node } from '@ui/types';

const BLOCK_HAVING_STATEMENTS = new Set(['IfStatement', 'ForStatement', 'ForInStatement', 'ForOfStatement', 'WhileStatement', 'DoWhileStatement', 'SwitchStatement', 'TryStatement']);

const TS_TYPE_DECLARATION_TYPES = new Set(['TSTypeAliasDeclaration', 'TSInterfaceDeclaration', 'TSEnumDeclaration', 'TSModuleDeclaration']);

const CLASS_METHOD_TYPES = new Set(['MethodDefinition', 'TSAbstractMethodDefinition']);

const CLASS_PROPERTY_TYPES = new Set(['PropertyDefinition', 'TSAbstractPropertyDefinition', 'AccessorProperty', 'TSIndexSignature', 'StaticBlock']);

const BLANK_LINE_ABOVE_TYPES = new Set(['SwitchStatement', 'SwitchCase', 'FunctionDeclaration', 'ClassDeclaration', 'TSEnumDeclaration', 'TSModuleDeclaration']);

function isExportWithDeclaration(n: Node): boolean {
	if (n.type !== 'ExportNamedDeclaration' && n.type !== 'ExportDefaultDeclaration') {
		return false;
	}

	return Boolean(n.declaration);
}

function isBlankLineAboveType(next: Node): boolean {
	return BLANK_LINE_ABOVE_TYPES.has(next.type);
}

function needsBlankLineAbove(next: Node): boolean {
	if (next.type === 'ReturnStatement') {
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

function isClassMethodPair(prev: Node, next: Node): boolean {
	return CLASS_METHOD_TYPES.has(prev.type) && CLASS_METHOD_TYPES.has(next.type);
}

function isPropertyToMethodTransition(prev: Node, next: Node): boolean {
	return CLASS_PROPERTY_TYPES.has(prev.type) && CLASS_METHOD_TYPES.has(next.type);
}

export function needsBlankLine(prev: Node, next: Node): boolean {
	if (needsBlankLineAbove(next)) {
		return true;
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
