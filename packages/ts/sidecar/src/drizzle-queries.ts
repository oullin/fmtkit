import { Ast } from '#sidecar/ast';
import { Edits } from '#sidecar/edits';
import { FileTargets } from '#sidecar/file-targets';
import { Node } from '#sidecar/node-schema';
import { isErr } from '#sidecar/result';
import { SourceText } from '#sidecar/source-text';
import { Sources } from '#sidecar/sources';
import type { Edit } from '#sidecar/types';

type DrizzleImports = {
	locals: Map<string, string>;
	namespaces: Set<string>;
};

// Detection: identify Drizzle imports, receivers, calls, and structural arguments.

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

/** Formats recognised Drizzle query structures without touching unrelated calls. */
export class DrizzleQueries {
	static #localName(node: Node | undefined): string | null {
		return node?.type === 'Identifier' ? (Ast.nodeName(node) ?? null) : null;
	}

	static #literalValue(node: Node | undefined): string | null {
		if (node?.type !== 'Literal') {
			return null;
		}

		return Ast.stringValue(node) ?? null;
	}

	static #propertyName(member: Node | undefined): string | null {
		if (member?.type !== 'MemberExpression' || member.computed) {
			return null;
		}

		return DrizzleQueries.#localName(Ast.childNode(member, 'property'));
	}

	static #calleeName(callee: Node | undefined, imports: DrizzleImports): string | null {
		if (!callee) {
			return null;
		}

		if (callee.type === 'Identifier') {
			const name = DrizzleQueries.#localName(callee);

			return name ? (imports.locals.get(name) ?? null) : null;
		}

		if (callee.type === 'MemberExpression' && !callee.computed) {
			const object = Ast.childNode(callee, 'object');

			const property = DrizzleQueries.#localName(Ast.childNode(callee, 'property'));

			const objectName = DrizzleQueries.#localName(object);

			if (objectName && property && imports.namespaces.has(objectName)) {
				return property;
			}
		}

		return null;
	}

	static #collectDrizzleImports(program: Node): DrizzleImports {
		const imports: DrizzleImports = { locals: new Map(), namespaces: new Set() };
		const body = Ast.childNodes(program, 'body');

		for (const statement of body) {
			if (statement.type !== 'ImportDeclaration') {
				continue;
			}

			const source = DrizzleQueries.#literalValue(Ast.childNode(statement, 'source'));

			if (!source?.startsWith(DRIZZLE_MODULE)) {
				continue;
			}

			for (const specifier of Ast.childNodes(statement, 'specifiers')) {
				if (specifier.type === 'ImportSpecifier') {
					const imported = DrizzleQueries.#localName(Ast.childNode(specifier, 'imported'));
					const local = DrizzleQueries.#localName(Ast.childNode(specifier, 'local'));

					if (imported && local) {
						imports.locals.set(local, imported);
					}
				}

				if (specifier.type === 'ImportNamespaceSpecifier') {
					const local = DrizzleQueries.#localName(Ast.childNode(specifier, 'local'));

					if (local) {
						imports.namespaces.add(local);
					}
				}
			}
		}

		return imports;
	}

	static #chainHasQueryMember(node: Node | undefined): boolean {
		const current = SourceText.unwrapChainExpression(node);

		if (!current) {
			return false;
		}

		if (current.type === 'MemberExpression') {
			if (DrizzleQueries.#propertyName(current) === 'query') {
				return true;
			}

			return DrizzleQueries.#chainHasQueryMember(Ast.childNode(current, 'object'));
		}

		if (current.type === 'CallExpression') {
			return DrizzleQueries.#chainHasQueryMember(Ast.childNode(current, 'callee'));
		}

		return false;
	}

	static #isDrizzleReceiver(node: Node | undefined, imports: DrizzleImports): boolean {
		const current = SourceText.unwrapChainExpression(node);

		if (!current) {
			return false;
		}

		if (current.type === 'Identifier') {
			const name = DrizzleQueries.#localName(current);

			return Boolean(name && DRIZZLE_RECEIVERS.has(name));
		}

		if (current.type === 'MemberExpression') {
			const object = Ast.childNode(current, 'object');
			const property = DrizzleQueries.#propertyName(current);

			if (property === 'query') {
				return DrizzleQueries.#isDrizzleReceiver(object, imports);
			}

			return DrizzleQueries.#isDrizzleReceiver(object, imports);
		}

		if (current.type === 'CallExpression') {
			const callee = SourceText.unwrapChainExpression(Ast.childNode(current, 'callee'));

			if (callee?.type === 'Identifier') {
				const imported = DrizzleQueries.#calleeName(callee, imports);

				return Boolean(imported && SET_OPERATION_HELPERS.has(imported));
			}

			if (callee?.type === 'MemberExpression') {
				const method = DrizzleQueries.#propertyName(callee);

				if (method && DRIZZLE_CHAIN_METHODS.has(method)) {
					return DrizzleQueries.#isDrizzleReceiver(Ast.childNode(callee, 'object'), imports);
				}

				return DrizzleQueries.#isDrizzleReceiver(Ast.childNode(callee, 'object'), imports);
			}
		}

		return false;
	}

	static #methodName(call: Node): string | null {
		const callee = SourceText.unwrapChainExpression(Ast.childNode(call, 'callee'));

		return callee?.type === 'MemberExpression' ? DrizzleQueries.#propertyName(callee) : null;
	}

	static #isDrizzleMethodCall(call: Node, imports: DrizzleImports): boolean {
		const callee = SourceText.unwrapChainExpression(Ast.childNode(call, 'callee'));

		if (callee?.type !== 'MemberExpression') {
			return false;
		}

		const name = DrizzleQueries.#propertyName(callee);

		if (!name || !DRIZZLE_FORMAT_METHODS.has(name)) {
			return false;
		}

		return DrizzleQueries.#isDrizzleReceiver(Ast.childNode(callee, 'object'), imports);
	}

	static #isRelationalQueryCall(call: Node, imports: DrizzleImports): boolean {
		const name = DrizzleQueries.#methodName(call);

		const callee = SourceText.unwrapChainExpression(Ast.childNode(call, 'callee'));

		if ((name !== 'findMany' && name !== 'findFirst') || callee?.type !== 'MemberExpression') {
			return false;
		}

		const object = Ast.childNode(callee, 'object');

		return DrizzleQueries.#chainHasQueryMember(object) && DrizzleQueries.#isDrizzleReceiver(object, imports);
	}

	static #isImportedHelperCall(call: Node, imports: DrizzleImports): boolean {
		const callee = SourceText.unwrapChainExpression(Ast.childNode(call, 'callee'));

		const name = DrizzleQueries.#calleeName(callee, imports);

		return Boolean(name && DRIZZLE_HELPERS.has(name));
	}

	static #isSetOperationCall(call: Node, imports: DrizzleImports): boolean {
		const callee = SourceText.unwrapChainExpression(Ast.childNode(call, 'callee'));

		const name = DrizzleQueries.#calleeName(callee, imports);

		return Boolean(name && SET_OPERATION_HELPERS.has(name));
	}

	static #callDisplayName(source: string, call: Node, imports: DrizzleImports): string {
		const callee = SourceText.unwrapChainExpression(Ast.childNode(call, 'callee'));

		if (callee?.type === 'Identifier') {
			return SourceText.sourceOf(source, callee);
		}

		if (callee?.type === 'MemberExpression') {
			const name = DrizzleQueries.#calleeName(callee, imports);

			if (name) {
				return SourceText.sourceOf(source, callee);
			}
		}

		return callee ? SourceText.sourceOf(source, callee) : '';
	}

	static #callParens(source: string, call: Node): { open: number; close: number } | null {
		return SourceText.callParens(source, call, SourceText.unwrapChainExpression(Ast.childNode(call, 'callee')));
	}

	static #shouldFormatObjectExpression(node: Node): boolean {
		const properties = Ast.childNodes(node, 'properties');

		if (properties.length > 1) {
			return true;
		}

		return properties.some((property) => {
			if (property.type !== 'Property') {
				return true;
			}

			const key = DrizzleQueries.#localName(Ast.childNode(property, 'key'));

			const value = Ast.childNode(property, 'value');

			if (!value) {
				return false;
			}

			if (key && DRIZZLE_OBJECT_KEYS.has(key) && (value.type === 'ObjectExpression' || value.type === 'ArrayExpression' || value.type === 'CallExpression')) {
				return true;
			}

			return value.type === 'ObjectExpression' || value.type === 'ArrayExpression';
		});
	}

	static #shouldFormatArrayExpression(node: Node): boolean {
		const elements = Array.isArray(node.elements) ? node.elements : [];

		return elements.length > 1 || elements.some((element) => element instanceof Node && (element.type === 'ObjectExpression' || element.type === 'CallExpression'));
	}

	static #isComplexArgument(node: Node, imports: DrizzleImports): boolean {
		if (node.type === 'ObjectExpression') {
			return DrizzleQueries.#shouldFormatObjectExpression(node);
		}

		if (node.type === 'ArrayExpression') {
			return DrizzleQueries.#shouldFormatArrayExpression(node);
		}

		if (node.type === 'CallExpression') {
			return DrizzleQueries.#isImportedHelperCall(node, imports) || DrizzleQueries.#isSetOperationCall(node, imports) || DrizzleQueries.#isDrizzleMethodCall(node, imports);
		}

		return false;
	}

	static #isStructuralArgument(node: Node, imports: DrizzleImports): boolean {
		if (node.type === 'ObjectExpression' || node.type === 'ArrayExpression') {
			return true;
		}

		return node.type === 'CallExpression' && (DrizzleQueries.#isSetOperationCall(node, imports) || DrizzleQueries.#isDrizzleMethodCall(node, imports));
	}

	static #shouldFormatMethodArguments(call: Node, imports: DrizzleImports): boolean {
		const args = Ast.childNodes(call, 'arguments');

		if (args.length === 0) {
			return false;
		}

		if (DrizzleQueries.#isRelationalQueryCall(call, imports)) {
			return args.some((arg) => arg.type === 'ObjectExpression' && DrizzleQueries.#shouldFormatObjectExpression(arg));
		}

		const name = DrizzleQueries.#methodName(call);

		if (!name) {
			return false;
		}

		if (['where', 'having', '$count'].includes(name)) {
			return args.some((arg) => DrizzleQueries.#isComplexArgument(arg, imports));
		}

		if (['leftJoin', 'rightJoin', 'innerJoin', 'fullJoin', 'crossJoin'].includes(name)) {
			return args.length > 1 && args.some((arg, index) => index > 0 && DrizzleQueries.#isComplexArgument(arg, imports));
		}

		if (['onConflictDoNothing', 'onConflictDoUpdate', 'returning', 'set', 'values'].includes(name)) {
			return args.some((arg) => DrizzleQueries.#isComplexArgument(arg, imports));
		}

		if (['as', 'except', 'groupBy', 'intersect', 'orderBy', 'union', 'unionAll'].includes(name)) {
			return args.length > 1 || args.some((arg) => DrizzleQueries.#isStructuralArgument(arg, imports));
		}

		return args.length > 1 && args.some((arg) => DrizzleQueries.#isComplexArgument(arg, imports));
	}

	// Emission: render recognised structures and produce non-overlapping edits.

	static #formatArrayExpression(source: string, node: Node, imports: DrizzleImports, comments: readonly Node[], indent: string, indentUnit: string): string {
		if (SourceText.hasCommentBetween(comments, Ast.getStart(node), Ast.getEnd(node))) {
			return SourceText.sourceOf(source, node);
		}

		const elements = Array.isArray(node.elements) ? node.elements : [];

		if (elements.length === 0) {
			return '[]';
		}

		const nextIndent = `${indent}${indentUnit}`;

		const formatted = elements.map((element) => {
			return element instanceof Node ? DrizzleQueries.#formatNode(source, element, imports, comments, nextIndent, indentUnit) : '';
		});

		return `[\n${nextIndent}${formatted.join(`,\n${nextIndent}`)},\n${indent}]`;
	}

	static #formatObjectExpression(source: string, node: Node, imports: DrizzleImports, comments: readonly Node[], indent: string, indentUnit: string): string {
		if (SourceText.hasCommentBetween(comments, Ast.getStart(node), Ast.getEnd(node))) {
			return SourceText.sourceOf(source, node);
		}

		const properties = Ast.childNodes(node, 'properties');

		if (properties.length === 0) {
			return '{}';
		}

		const nextIndent = `${indent}${indentUnit}`;

		const formatted = properties.map((property) => {
			if (property.type !== 'Property') {
				return SourceText.sourceOf(source, property);
			}

			const key = Ast.childNode(property, 'key');
			const value = Ast.childNode(property, 'value');

			if (!key || !value || property.computed || property.method) {
				return SourceText.sourceOf(source, property);
			}

			if (property.shorthand) {
				return SourceText.sourceOf(source, property);
			}

			return `${SourceText.sourceOf(source, key)}: ${DrizzleQueries.#formatNode(source, value, imports, comments, nextIndent, indentUnit)}`;
		});

		return `{\n${nextIndent}${formatted.join(`,\n${nextIndent}`)},\n${indent}}`;
	}

	static #formatHelperCall(source: string, call: Node, imports: DrizzleImports, comments: readonly Node[], indent: string, indentUnit: string): string {
		if (SourceText.hasCommentBetween(comments, Ast.getStart(call), Ast.getEnd(call))) {
			return SourceText.sourceOf(source, call);
		}

		const importedName = DrizzleQueries.#calleeName(SourceText.unwrapChainExpression(Ast.childNode(call, 'callee')), imports);

		const args = Ast.childNodes(call, 'arguments');

		if (!importedName || !MULTILINE_HELPERS.has(importedName) || args.length === 0) {
			return SourceText.sourceOf(source, call);
		}

		const nextIndent = `${indent}${indentUnit}`;
		const formatted = args.map((arg) => DrizzleQueries.#formatNode(source, arg, imports, comments, nextIndent, indentUnit));

		return `${DrizzleQueries.#callDisplayName(source, call, imports)}(\n${nextIndent}${formatted.join(`,\n${nextIndent}`)},\n${indent})`;
	}

	static #formatSetOperationCall(source: string, call: Node, imports: DrizzleImports, comments: readonly Node[], indent: string, indentUnit: string): string {
		if (SourceText.hasCommentBetween(comments, Ast.getStart(call), Ast.getEnd(call))) {
			return SourceText.sourceOf(source, call);
		}

		const args = Ast.childNodes(call, 'arguments');

		if (args.length < 2) {
			return SourceText.sourceOf(source, call);
		}

		const nextIndent = `${indent}${indentUnit}`;
		const formatted = args.map((arg) => DrizzleQueries.#formatNode(source, arg, imports, comments, nextIndent, indentUnit));

		return `${DrizzleQueries.#callDisplayName(source, call, imports)}(\n${nextIndent}${formatted.join(`,\n${nextIndent}`)},\n${indent})`;
	}

	static #formatNode(source: string, node: Node, imports: DrizzleImports, comments: readonly Node[], indent: string, indentUnit: string): string {
		if (node.type === 'ObjectExpression' && DrizzleQueries.#shouldFormatObjectExpression(node)) {
			return DrizzleQueries.#formatObjectExpression(source, node, imports, comments, indent, indentUnit);
		}

		if (node.type === 'ArrayExpression' && DrizzleQueries.#shouldFormatArrayExpression(node)) {
			return DrizzleQueries.#formatArrayExpression(source, node, imports, comments, indent, indentUnit);
		}

		if (node.type === 'CallExpression') {
			if (DrizzleQueries.#isSetOperationCall(node, imports)) {
				return DrizzleQueries.#formatSetOperationCall(source, node, imports, comments, indent, indentUnit);
			}

			if (DrizzleQueries.#isImportedHelperCall(node, imports)) {
				return DrizzleQueries.#formatHelperCall(source, node, imports, comments, indent, indentUnit);
			}
		}

		return SourceText.sourceOf(source, node);
	}

	static #formatCallArguments(source: string, call: Node, imports: DrizzleImports, comments: readonly Node[], indentUnit: string): Edit | null {
		const parens = DrizzleQueries.#callParens(source, call);
		const args = Ast.childNodes(call, 'arguments');

		if (!parens || args.length === 0) {
			return null;
		}

		if (SourceText.hasCommentBetween(comments, parens.open, parens.close)) {
			return null;
		}

		const callee = SourceText.unwrapChainExpression(Ast.childNode(call, 'callee'));

		const property = callee ? Ast.childNode(callee, 'property') : undefined;
		const indentPos = callee?.type === 'MemberExpression' && property ? Ast.getStart(property) : Ast.getStart(call);
		const indent = SourceText.lineIndent(source, indentPos);
		const argIndent = `${indent}${indentUnit}`;
		const formatted = args.map((arg) => DrizzleQueries.#formatNode(source, arg, imports, comments, argIndent, indentUnit));
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

	/**
	 * Compute edits for recognised Drizzle query structures.
	 *
	 * @param content - The source text to inspect.
	 * @param virtualName - The filename used to parse the source.
	 * @returns Non-overlapping query-formatting edits.
	 */
	static computeEdits(content: string, virtualName: string): Edit[] {
		if (FileTargets.isDeclarationFile(virtualName)) {
			return [];
		}

		const parsed = Sources.parse(virtualName, content);

		if (isErr(parsed)) {
			return [];
		}

		const comments = parsed.value.comments;
		const imports = DrizzleQueries.#collectDrizzleImports(parsed.value.program);

		if (imports.locals.size === 0 && imports.namespaces.size === 0) {
			return [];
		}

		const edits: Edit[] = [];
		const indentUnit = SourceText.detectIndentUnit(content);

		Ast.visit(parsed.value.program, (node) => {
			if (node.type !== 'CallExpression') {
				return;
			}

			if (DrizzleQueries.#isDrizzleMethodCall(node, imports) || DrizzleQueries.#isRelationalQueryCall(node, imports) || DrizzleQueries.#isSetOperationCall(node, imports)) {
				const args = Ast.childNodes(node, 'arguments');

				if (DrizzleQueries.#isSetOperationCall(node, imports) && args.length > 0 && args.length < 2) {
					return;
				}

				if (!DrizzleQueries.#isSetOperationCall(node, imports) && !DrizzleQueries.#shouldFormatMethodArguments(node, imports)) {
					return;
				}

				const edit = DrizzleQueries.#formatCallArguments(content, node, imports, comments, indentUnit);

				if (edit) {
					edits.push(edit);
				}
			}
		});

		return Edits.nonOverlapping(edits);
	}

	/**
	 * Format recognised Drizzle query structures.
	 *
	 * @param content - The source text to format.
	 * @param virtualName - The filename used to parse the source.
	 * @returns The formatted source, or the original source when no edits apply.
	 */
	static format(content: string, virtualName: string): string {
		const edits = DrizzleQueries.computeEdits(content, virtualName);

		return edits.length > 0 ? Edits.apply(content, edits) : content;
	}
}
