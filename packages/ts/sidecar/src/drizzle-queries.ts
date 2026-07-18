import { childNode, childNodes, getEnd, getStart, isNode, nodeName, visit } from '#sidecar/ast';
import { applyEdits } from '#sidecar/edits';
import { callParens as sharedCallParens, hasCommentBetween, isDeclarationFile, lineIndent, nonOverlappingEdits, parseCleanly, sourceOf, unwrapChainExpression } from '#sidecar/pass-utils';
import type { Edit, Node } from '#sidecar/types';

type DrizzleImports = {
	locals: Map<string, string>;
	namespaces: Set<string>;
};

const DRIZZLE_MODULE = 'drizzle-orm';
const DRIZZLE_RECEIVERS = new Set(['db', 'tx']);

const DRIZZLE_CHAIN_METHODS = new Set([
	'$count',
	'$dynamic',
	'$with',
	'as',
	'crossJoin',
	'delete',
	'except',
	'from',
	'fullJoin',
	'groupBy',
	'having',
	'innerJoin',
	'insert',
	'intersect',
	'leftJoin',
	'limit',
	'offset',
	'onConflictDoNothing',
	'onConflictDoUpdate',
	'orderBy',
	'prepare',
	'returning',
	'rightJoin',
	'select',
	'set',
	'union',
	'unionAll',
	'update',
	'values',
	'where',
	'with',
]);

const DRIZZLE_FORMAT_METHODS = new Set([
	'$count',
	'as',
	'crossJoin',
	'except',
	'findFirst',
	'findMany',
	'fullJoin',
	'groupBy',
	'having',
	'innerJoin',
	'intersect',
	'leftJoin',
	'onConflictDoNothing',
	'onConflictDoUpdate',
	'orderBy',
	'returning',
	'rightJoin',
	'set',
	'union',
	'unionAll',
	'values',
	'where',
]);

const DRIZZLE_HELPERS = new Set([
	'and',
	'arrayContained',
	'arrayContains',
	'arrayOverlaps',
	'asc',
	'between',
	'desc',
	'eq',
	'exists',
	'gt',
	'gte',
	'ilike',
	'inArray',
	'isNotNull',
	'isNull',
	'like',
	'lt',
	'lte',
	'ne',
	'not',
	'notBetween',
	'notExists',
	'notIlike',
	'notInArray',
	'notLike',
	'or',
	'sql',
]);

const MULTILINE_HELPERS = new Set(['and', 'or', 'not', 'exists', 'notExists']);
const SET_OPERATION_HELPERS = new Set(['except', 'intersect', 'union', 'unionAll']);
const DRIZZLE_OBJECT_KEYS = new Set(['columns', 'extras', 'limit', 'offset', 'onUpdate', 'orderBy', 'set', 'target', 'targetWhere', 'where', 'with']);

function localName(node: Node | undefined): string | null {
	return node?.type === 'Identifier' ? (nodeName(node) ?? null) : null;
}

function literalValue(node: Node | undefined): string | null {
	if (node?.type !== 'Literal') {
		return null;
	}

	const value = node.value;

	return typeof value === 'string' ? value : null;
}

function propertyName(member: Node | undefined): string | null {
	if (member?.type !== 'MemberExpression' || member.computed) {
		return null;
	}

	return localName(
		childNode(member, 'property'),
	);
}

function calleeName(callee: Node | undefined, imports: DrizzleImports): string | null {
	if (!callee) {
		return null;
	}

	if (callee.type === 'Identifier') {
		const name = localName(callee);

		return name ? (imports.locals.get(name) ?? null) : null;
	}

	if (callee.type === 'MemberExpression' && !callee.computed) {
		const object = childNode(callee, 'object');

		const property = localName(
			childNode(callee, 'property'),
		);

		const objectName = localName(object);

		if (objectName && property && imports.namespaces.has(objectName)) {
			return property;
		}
	}

	return null;
}

function collectDrizzleImports(program: Node): DrizzleImports {
	const imports: DrizzleImports = { locals: new Map(), namespaces: new Set() };
	const body = childNodes(program, 'body');

	for (const statement of body) {
		if (statement.type !== 'ImportDeclaration') {
			continue;
		}

		const source = literalValue(
			childNode(statement, 'source'),
		);

		if (!source?.startsWith(DRIZZLE_MODULE)) {
			continue;
		}

		for (const specifier of childNodes(statement, 'specifiers')) {
			if (specifier.type === 'ImportSpecifier') {
				const imported = localName(
					childNode(specifier, 'imported'),
				);
				const local = localName(
					childNode(specifier, 'local'),
				);

				if (imported && local) {
					imports.locals.set(local, imported);
				}
			}

			if (specifier.type === 'ImportNamespaceSpecifier') {
				const local = localName(
					childNode(specifier, 'local'),
				);

				if (local) {
					imports.namespaces.add(local);
				}
			}
		}
	}

	return imports;
}

function chainHasQueryMember(node: Node | undefined): boolean {
	const current = unwrapChainExpression(node);

	if (!current) {
		return false;
	}

	if (current.type === 'MemberExpression') {
		if (propertyName(current) === 'query') {
			return true;
		}

		return chainHasQueryMember(
			childNode(current, 'object'),
		);
	}

	if (current.type === 'CallExpression') {
		return chainHasQueryMember(
			childNode(current, 'callee'),
		);
	}

	return false;
}

function isDrizzleReceiver(node: Node | undefined, imports: DrizzleImports): boolean {
	const current = unwrapChainExpression(node);

	if (!current) {
		return false;
	}

	if (current.type === 'Identifier') {
		const name = localName(current);

		return Boolean(name && DRIZZLE_RECEIVERS.has(name));
	}

	if (current.type === 'MemberExpression') {
		const object = childNode(current, 'object');
		const property = propertyName(current);

		if (property === 'query') {
			return isDrizzleReceiver(object, imports);
		}

		return isDrizzleReceiver(object, imports);
	}

	if (current.type === 'CallExpression') {
		const callee = unwrapChainExpression(
			childNode(current, 'callee'),
		);

		if (callee?.type === 'Identifier') {
			const imported = calleeName(callee, imports);

			return Boolean(imported && SET_OPERATION_HELPERS.has(imported));
		}

		if (callee?.type === 'MemberExpression') {
			const method = propertyName(callee);

			if (method && DRIZZLE_CHAIN_METHODS.has(method)) {
				return isDrizzleReceiver(
					childNode(callee, 'object'),
					imports,
				);
			}

			return isDrizzleReceiver(
				childNode(callee, 'object'),
				imports,
			);
		}
	}

	return false;
}

function methodName(call: Node): string | null {
	const callee = unwrapChainExpression(
		childNode(call, 'callee'),
	);

	return callee?.type === 'MemberExpression' ? propertyName(callee) : null;
}

function isDrizzleMethodCall(call: Node, imports: DrizzleImports): boolean {
	const callee = unwrapChainExpression(
		childNode(call, 'callee'),
	);

	if (callee?.type !== 'MemberExpression') {
		return false;
	}

	const name = propertyName(callee);

	if (!name || !DRIZZLE_FORMAT_METHODS.has(name)) {
		return false;
	}

	return isDrizzleReceiver(
		childNode(callee, 'object'),
		imports,
	);
}

function isRelationalQueryCall(call: Node, imports: DrizzleImports): boolean {
	const name = methodName(call);

	const callee = unwrapChainExpression(
		childNode(call, 'callee'),
	);

	if ((name !== 'findMany' && name !== 'findFirst') || callee?.type !== 'MemberExpression') {
		return false;
	}

	const object = childNode(callee, 'object');

	return chainHasQueryMember(object) && isDrizzleReceiver(object, imports);
}

function isImportedHelperCall(call: Node, imports: DrizzleImports): boolean {
	const callee = unwrapChainExpression(
		childNode(call, 'callee'),
	);

	const name = calleeName(callee, imports);

	return Boolean(name && DRIZZLE_HELPERS.has(name));
}

function isSetOperationCall(call: Node, imports: DrizzleImports): boolean {
	const callee = unwrapChainExpression(
		childNode(call, 'callee'),
	);

	const name = calleeName(callee, imports);

	return Boolean(name && SET_OPERATION_HELPERS.has(name));
}

function callDisplayName(source: string, call: Node, imports: DrizzleImports): string {
	const callee = unwrapChainExpression(
		childNode(call, 'callee'),
	);

	if (callee?.type === 'Identifier') {
		return sourceOf(source, callee);
	}

	if (callee?.type === 'MemberExpression') {
		const name = calleeName(callee, imports);

		if (name) {
			return sourceOf(source, callee);
		}
	}

	return callee ? sourceOf(source, callee) : '';
}

function callParens(source: string, call: Node): { open: number; close: number } | null {
	return sharedCallParens(
		source,
		call,
		unwrapChainExpression(
			childNode(call, 'callee'),
		),
	);
}

function shouldFormatObjectExpression(node: Node): boolean {
	const properties = childNodes(node, 'properties');

	if (properties.length > 1) {
		return true;
	}

	return properties.some((property) => {
		if (property.type !== 'Property') {
			return true;
		}

		const key = localName(
			childNode(property, 'key'),
		);

		const value = childNode(property, 'value');

		if (!value) {
			return false;
		}

		if (key && DRIZZLE_OBJECT_KEYS.has(key) && (value.type === 'ObjectExpression' || value.type === 'ArrayExpression' || value.type === 'CallExpression')) {
			return true;
		}

		return value.type === 'ObjectExpression' || value.type === 'ArrayExpression';
	});
}

function shouldFormatArrayExpression(node: Node): boolean {
	const elements = Array.isArray(node.elements) ? node.elements : [];

	return elements.length > 1 || elements.some((element) => Boolean(isNode(element) && (element.type === 'ObjectExpression' || element.type === 'CallExpression')));
}

function isComplexArgument(node: Node, imports: DrizzleImports): boolean {
	if (node.type === 'ObjectExpression') {
		return shouldFormatObjectExpression(node);
	}

	if (node.type === 'ArrayExpression') {
		return shouldFormatArrayExpression(node);
	}

	if (node.type === 'CallExpression') {
		return isImportedHelperCall(node, imports) || isSetOperationCall(node, imports) || isDrizzleMethodCall(node, imports);
	}

	return false;
}

function isStructuralArgument(node: Node, imports: DrizzleImports): boolean {
	if (node.type === 'ObjectExpression' || node.type === 'ArrayExpression') {
		return true;
	}

	return node.type === 'CallExpression' && (isSetOperationCall(node, imports) || isDrizzleMethodCall(node, imports));
}

function shouldFormatMethodArguments(call: Node, imports: DrizzleImports): boolean {
	const args = childNodes(call, 'arguments');

	if (args.length === 0) {
		return false;
	}

	if (isRelationalQueryCall(call, imports)) {
		return args.some((arg) => arg.type === 'ObjectExpression' && shouldFormatObjectExpression(arg));
	}

	const name = methodName(call);

	if (!name) {
		return false;
	}

	if (['where', 'having', '$count'].includes(name)) {
		return args.some((arg) => isComplexArgument(arg, imports));
	}

	if (['leftJoin', 'rightJoin', 'innerJoin', 'fullJoin', 'crossJoin'].includes(name)) {
		return args.length > 1 && args.some((arg, index) => index > 0 && isComplexArgument(arg, imports));
	}

	if (['onConflictDoNothing', 'onConflictDoUpdate', 'returning', 'set', 'values'].includes(name)) {
		return args.some((arg) => isComplexArgument(arg, imports));
	}

	if (['as', 'except', 'groupBy', 'intersect', 'orderBy', 'union', 'unionAll'].includes(name)) {
		return args.length > 1 || args.some((arg) => isStructuralArgument(arg, imports));
	}

	return args.length > 1 && args.some((arg) => isComplexArgument(arg, imports));
}

function formatArrayExpression(source: string, node: Node, imports: DrizzleImports, comments: Node[], indent: string): string {
	if (hasCommentBetween(
		comments,
		getStart(node),
		getEnd(node),
	)) {
		return sourceOf(source, node);
	}

	const elements = Array.isArray(node.elements) ? node.elements : [];

	if (elements.length === 0) {
		return '[]';
	}

	const nextIndent = `${indent}\t`;

	const formatted = elements.map((element) => {
		return isNode(element) ? formatNode(source, element, imports, comments, nextIndent) : '';
	});

	return `[\n${nextIndent}${formatted.join(`,\n${nextIndent}`)},\n${indent}]`;
}

function formatObjectExpression(source: string, node: Node, imports: DrizzleImports, comments: Node[], indent: string): string {
	if (hasCommentBetween(
		comments,
		getStart(node),
		getEnd(node),
	)) {
		return sourceOf(source, node);
	}

	const properties = childNodes(node, 'properties');

	if (properties.length === 0) {
		return '{}';
	}

	const nextIndent = `${indent}\t`;

	const formatted = properties.map((property) => {
		if (property.type !== 'Property') {
			return sourceOf(source, property);
		}

		const key = childNode(property, 'key');
		const value = childNode(property, 'value');

		if (!key || !value || property.computed || property.method) {
			return sourceOf(source, property);
		}

		if (property.shorthand) {
			return sourceOf(source, property);
		}

		return `${sourceOf(source, key)}: ${formatNode(source, value, imports, comments, nextIndent)}`;
	});

	return `{\n${nextIndent}${formatted.join(`,\n${nextIndent}`)},\n${indent}}`;
}

function formatHelperCall(source: string, call: Node, imports: DrizzleImports, comments: Node[], indent: string): string {
	if (hasCommentBetween(
		comments,
		getStart(call),
		getEnd(call),
	)) {
		return sourceOf(source, call);
	}

	const importedName = calleeName(
		unwrapChainExpression(
			childNode(call, 'callee'),
		),
		imports,
	);

	const args = childNodes(call, 'arguments');

	if (!importedName || !MULTILINE_HELPERS.has(importedName) || args.length === 0) {
		return sourceOf(source, call);
	}

	const nextIndent = `${indent}\t`;
	const formatted = args.map((arg) => formatNode(source, arg, imports, comments, nextIndent));

	return `${callDisplayName(source, call, imports)}(\n${nextIndent}${formatted.join(`,\n${nextIndent}`)},\n${indent})`;
}

function formatSetOperationCall(source: string, call: Node, imports: DrizzleImports, comments: Node[], indent: string): string {
	if (hasCommentBetween(
		comments,
		getStart(call),
		getEnd(call),
	)) {
		return sourceOf(source, call);
	}

	const args = childNodes(call, 'arguments');

	if (args.length < 2) {
		return sourceOf(source, call);
	}

	const nextIndent = `${indent}\t`;
	const formatted = args.map((arg) => formatNode(source, arg, imports, comments, nextIndent));

	return `${callDisplayName(source, call, imports)}(\n${nextIndent}${formatted.join(`,\n${nextIndent}`)},\n${indent})`;
}

function formatNode(source: string, node: Node, imports: DrizzleImports, comments: Node[], indent: string): string {
	if (node.type === 'ObjectExpression' && shouldFormatObjectExpression(node)) {
		return formatObjectExpression(source, node, imports, comments, indent);
	}

	if (node.type === 'ArrayExpression' && shouldFormatArrayExpression(node)) {
		return formatArrayExpression(source, node, imports, comments, indent);
	}

	if (node.type === 'CallExpression') {
		if (isSetOperationCall(node, imports)) {
			return formatSetOperationCall(source, node, imports, comments, indent);
		}

		if (isImportedHelperCall(node, imports)) {
			return formatHelperCall(source, node, imports, comments, indent);
		}
	}

	return sourceOf(source, node);
}

function formatCallArguments(source: string, call: Node, imports: DrizzleImports, comments: Node[]): Edit | null {
	const parens = callParens(source, call);
	const args = childNodes(call, 'arguments');

	if (!parens || args.length === 0) {
		return null;
	}

	if (hasCommentBetween(comments, parens.open, parens.close)) {
		return null;
	}

	const callee = unwrapChainExpression(
		childNode(call, 'callee'),
	);

	const property = callee ? childNode(callee, 'property') : undefined;
	const indentPos = callee?.type === 'MemberExpression' && property ? getStart(property) : getStart(call);
	const indent = lineIndent(source, indentPos);
	const argIndent = `${indent}\t`;
	const formatted = args.map((arg) => formatNode(source, arg, imports, comments, argIndent));
	const replacement = `(\n${argIndent}${formatted.join(`,\n${argIndent}`)},\n${indent})`;

	if (source.slice(parens.open, parens.close + 1) === replacement) {
		return null;
	}

	return {
		start: parens.open,
		end: parens.close + 1,
		replacement,
	};
}

export function computeDrizzleQueryEdits(content: string, virtualName: string): Edit[] {
	if (isDeclarationFile(virtualName)) {
		return [];
	}

	const parsed = parseCleanly(virtualName, content);

	if (!parsed) {
		return [];
	}

	const comments = parsed.comments;
	const imports = collectDrizzleImports(parsed.program);

	if (imports.locals.size === 0 && imports.namespaces.size === 0) {
		return [];
	}

	const edits: Edit[] = [];

	visit(parsed.program, (node) => {
		if (node.type !== 'CallExpression') {
			return;
		}

		if (isDrizzleMethodCall(node, imports) || isRelationalQueryCall(node, imports) || isSetOperationCall(node, imports)) {
			const args = childNodes(node, 'arguments');

			if (isSetOperationCall(node, imports) && args.length > 0 && args.length < 2) {
				return;
			}

			if (!isSetOperationCall(node, imports) && !shouldFormatMethodArguments(node, imports)) {
				return;
			}

			const edit = formatCallArguments(content, node, imports, comments);

			if (edit) {
				edits.push(edit);
			}
		}
	});

	return nonOverlappingEdits(edits);
}

export function formatDrizzleQueries(content: string, virtualName: string): string {
	const edits = computeDrizzleQueryEdits(content, virtualName);

	return edits.length > 0 ? applyEdits(content, edits) : content;
}
