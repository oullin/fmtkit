import { AstReader } from '#sidecar/syntax/ast-reader';
import { FileTargets } from '#sidecar/hosts/file-targets';
import { Node } from '#sidecar/syntax/node-schema';
import type { ParsedSourceDto } from '#sidecar/syntax/node-schema';
import { isErr } from '#sidecar/kernel/result';
import type { Edit, EditApplier } from '#sidecar/syntax/edits';
import type { FormattingPass } from '#sidecar/passes/pass';
import type { SourceDocument } from '#sidecar/syntax/source-document';
import type { SourceParser } from '#sidecar/syntax/source-parser';

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

/**
 * Formats recognised Drizzle query structures without touching unrelated calls.
 *
 * The detection and emission internals remain static pending the TS-5 split; this
 * pass injects the parser and edit reducer at its boundary and delegates the rest
 * to the preserved static helpers, which still share a single `AstReader`.
 */
export class DrizzleQueryPass implements FormattingPass {
	/** The pass identity used for reporting. */
	readonly name = 'drizzle-queries';

	static readonly #ast = new AstReader();

	readonly #parser: SourceParser;
	readonly #edits: EditApplier;

	/**
	 * @param dependencies - The syntax services consumed by the pass.
	 * @param dependencies.parser - Parses source into a trustworthy tree.
	 * @param dependencies.edits - Reduces candidate edits to a non-overlapping set.
	 */
	constructor(dependencies: { parser: SourceParser; edits: EditApplier }) {
		this.#parser = dependencies.parser;
		this.#edits = dependencies.edits;
	}

	static #localName(node: Node | undefined): string | null {
		return node?.type === 'Identifier' ? (DrizzleQueryPass.#ast.nodeName(node) ?? null) : null;
	}

	static #literalValue(node: Node | undefined): string | null {
		if (node?.type !== 'Literal') {
			return null;
		}

		return DrizzleQueryPass.#ast.stringValue(node) ?? null;
	}

	static #propertyName(member: Node | undefined): string | null {
		if (member?.type !== 'MemberExpression' || member.computed) {
			return null;
		}

		return DrizzleQueryPass.#localName(DrizzleQueryPass.#ast.childNode(member, 'property'));
	}

	static #calleeName(callee: Node | undefined, imports: DrizzleImports): string | null {
		if (!callee) {
			return null;
		}

		if (callee.type === 'Identifier') {
			const name = DrizzleQueryPass.#localName(callee);

			return name ? (imports.locals.get(name) ?? null) : null;
		}

		if (callee.type === 'MemberExpression' && !callee.computed) {
			const object = DrizzleQueryPass.#ast.childNode(callee, 'object');

			const property = DrizzleQueryPass.#localName(DrizzleQueryPass.#ast.childNode(callee, 'property'));

			const objectName = DrizzleQueryPass.#localName(object);

			if (objectName && property && imports.namespaces.has(objectName)) {
				return property;
			}
		}

		return null;
	}

	static #collectDrizzleImports(program: Node): DrizzleImports {
		const imports: DrizzleImports = { locals: new Map(), namespaces: new Set() };
		const body = DrizzleQueryPass.#ast.childNodes(program, 'body');

		for (const statement of body) {
			if (statement.type !== 'ImportDeclaration') {
				continue;
			}

			const source = DrizzleQueryPass.#literalValue(DrizzleQueryPass.#ast.childNode(statement, 'source'));

			if (!source?.startsWith(DRIZZLE_MODULE)) {
				continue;
			}

			for (const specifier of DrizzleQueryPass.#ast.childNodes(statement, 'specifiers')) {
				if (specifier.type === 'ImportSpecifier') {
					const imported = DrizzleQueryPass.#localName(DrizzleQueryPass.#ast.childNode(specifier, 'imported'));
					const local = DrizzleQueryPass.#localName(DrizzleQueryPass.#ast.childNode(specifier, 'local'));

					if (imported && local) {
						imports.locals.set(local, imported);
					}
				}

				if (specifier.type === 'ImportNamespaceSpecifier') {
					const local = DrizzleQueryPass.#localName(DrizzleQueryPass.#ast.childNode(specifier, 'local'));

					if (local) {
						imports.namespaces.add(local);
					}
				}
			}
		}

		return imports;
	}

	static #chainHasQueryMember(node: Node | undefined): boolean {
		const current = DrizzleQueryPass.#ast.unwrapChainExpression(node);

		if (!current) {
			return false;
		}

		if (current.type === 'MemberExpression') {
			if (DrizzleQueryPass.#propertyName(current) === 'query') {
				return true;
			}

			return DrizzleQueryPass.#chainHasQueryMember(DrizzleQueryPass.#ast.childNode(current, 'object'));
		}

		if (current.type === 'CallExpression') {
			return DrizzleQueryPass.#chainHasQueryMember(DrizzleQueryPass.#ast.childNode(current, 'callee'));
		}

		return false;
	}

	static #isDrizzleReceiver(node: Node | undefined, imports: DrizzleImports): boolean {
		const current = DrizzleQueryPass.#ast.unwrapChainExpression(node);

		if (!current) {
			return false;
		}

		if (current.type === 'Identifier') {
			const name = DrizzleQueryPass.#localName(current);

			return Boolean(name && DRIZZLE_RECEIVERS.has(name));
		}

		if (current.type === 'MemberExpression') {
			const object = DrizzleQueryPass.#ast.childNode(current, 'object');
			const property = DrizzleQueryPass.#propertyName(current);

			if (property === 'query') {
				return DrizzleQueryPass.#isDrizzleReceiver(object, imports);
			}

			return DrizzleQueryPass.#isDrizzleReceiver(object, imports);
		}

		if (current.type === 'CallExpression') {
			const callee = DrizzleQueryPass.#ast.unwrapChainExpression(DrizzleQueryPass.#ast.childNode(current, 'callee'));

			if (callee?.type === 'Identifier') {
				const imported = DrizzleQueryPass.#calleeName(callee, imports);

				return Boolean(imported && SET_OPERATION_HELPERS.has(imported));
			}

			if (callee?.type === 'MemberExpression') {
				const method = DrizzleQueryPass.#propertyName(callee);

				if (method && DRIZZLE_CHAIN_METHODS.has(method)) {
					return DrizzleQueryPass.#isDrizzleReceiver(DrizzleQueryPass.#ast.childNode(callee, 'object'), imports);
				}

				return DrizzleQueryPass.#isDrizzleReceiver(DrizzleQueryPass.#ast.childNode(callee, 'object'), imports);
			}
		}

		return false;
	}

	static #methodName(call: Node): string | null {
		const callee = DrizzleQueryPass.#ast.unwrapChainExpression(DrizzleQueryPass.#ast.childNode(call, 'callee'));

		return callee?.type === 'MemberExpression' ? DrizzleQueryPass.#propertyName(callee) : null;
	}

	static #isDrizzleMethodCall(call: Node, imports: DrizzleImports): boolean {
		const callee = DrizzleQueryPass.#ast.unwrapChainExpression(DrizzleQueryPass.#ast.childNode(call, 'callee'));

		if (callee?.type !== 'MemberExpression') {
			return false;
		}

		const name = DrizzleQueryPass.#propertyName(callee);

		if (!name || !DRIZZLE_FORMAT_METHODS.has(name)) {
			return false;
		}

		return DrizzleQueryPass.#isDrizzleReceiver(DrizzleQueryPass.#ast.childNode(callee, 'object'), imports);
	}

	static #isRelationalQueryCall(call: Node, imports: DrizzleImports): boolean {
		const name = DrizzleQueryPass.#methodName(call);

		const callee = DrizzleQueryPass.#ast.unwrapChainExpression(DrizzleQueryPass.#ast.childNode(call, 'callee'));

		if ((name !== 'findMany' && name !== 'findFirst') || callee?.type !== 'MemberExpression') {
			return false;
		}

		const object = DrizzleQueryPass.#ast.childNode(callee, 'object');

		return DrizzleQueryPass.#chainHasQueryMember(object) && DrizzleQueryPass.#isDrizzleReceiver(object, imports);
	}

	static #isImportedHelperCall(call: Node, imports: DrizzleImports): boolean {
		const callee = DrizzleQueryPass.#ast.unwrapChainExpression(DrizzleQueryPass.#ast.childNode(call, 'callee'));

		const name = DrizzleQueryPass.#calleeName(callee, imports);

		return Boolean(name && DRIZZLE_HELPERS.has(name));
	}

	static #isSetOperationCall(call: Node, imports: DrizzleImports): boolean {
		const callee = DrizzleQueryPass.#ast.unwrapChainExpression(DrizzleQueryPass.#ast.childNode(call, 'callee'));

		const name = DrizzleQueryPass.#calleeName(callee, imports);

		return Boolean(name && SET_OPERATION_HELPERS.has(name));
	}

	static #callDisplayName(source: string, call: Node, imports: DrizzleImports): string {
		const callee = DrizzleQueryPass.#ast.unwrapChainExpression(DrizzleQueryPass.#ast.childNode(call, 'callee'));

		if (callee?.type === 'Identifier') {
			return DrizzleQueryPass.#ast.sourceOf(source, callee);
		}

		if (callee?.type === 'MemberExpression') {
			const name = DrizzleQueryPass.#calleeName(callee, imports);

			if (name) {
				return DrizzleQueryPass.#ast.sourceOf(source, callee);
			}
		}

		return callee ? DrizzleQueryPass.#ast.sourceOf(source, callee) : '';
	}

	static #callParens(source: string, call: Node): { open: number; close: number } | null {
		return DrizzleQueryPass.#ast.callParens(source, call, DrizzleQueryPass.#ast.unwrapChainExpression(DrizzleQueryPass.#ast.childNode(call, 'callee')));
	}

	static #shouldFormatObjectExpression(node: Node): boolean {
		const properties = DrizzleQueryPass.#ast.childNodes(node, 'properties');

		if (properties.length > 1) {
			return true;
		}

		return properties.some((property) => {
			if (property.type !== 'Property') {
				return true;
			}

			const key = DrizzleQueryPass.#localName(DrizzleQueryPass.#ast.childNode(property, 'key'));

			const value = DrizzleQueryPass.#ast.childNode(property, 'value');

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
			return DrizzleQueryPass.#shouldFormatObjectExpression(node);
		}

		if (node.type === 'ArrayExpression') {
			return DrizzleQueryPass.#shouldFormatArrayExpression(node);
		}

		if (node.type === 'CallExpression') {
			return DrizzleQueryPass.#isImportedHelperCall(node, imports) || DrizzleQueryPass.#isSetOperationCall(node, imports) || DrizzleQueryPass.#isDrizzleMethodCall(node, imports);
		}

		return false;
	}

	static #isStructuralArgument(node: Node, imports: DrizzleImports): boolean {
		if (node.type === 'ObjectExpression' || node.type === 'ArrayExpression') {
			return true;
		}

		return node.type === 'CallExpression' && (DrizzleQueryPass.#isSetOperationCall(node, imports) || DrizzleQueryPass.#isDrizzleMethodCall(node, imports));
	}

	static #shouldFormatMethodArguments(call: Node, imports: DrizzleImports): boolean {
		const args = DrizzleQueryPass.#ast.childNodes(call, 'arguments');

		if (args.length === 0) {
			return false;
		}

		if (DrizzleQueryPass.#isRelationalQueryCall(call, imports)) {
			return args.some((arg) => arg.type === 'ObjectExpression' && DrizzleQueryPass.#shouldFormatObjectExpression(arg));
		}

		const name = DrizzleQueryPass.#methodName(call);

		if (!name) {
			return false;
		}

		if (['where', 'having', '$count'].includes(name)) {
			return args.some((arg) => DrizzleQueryPass.#isComplexArgument(arg, imports));
		}

		if (['leftJoin', 'rightJoin', 'innerJoin', 'fullJoin', 'crossJoin'].includes(name)) {
			return args.length > 1 && args.some((arg, index) => index > 0 && DrizzleQueryPass.#isComplexArgument(arg, imports));
		}

		if (['onConflictDoNothing', 'onConflictDoUpdate', 'returning', 'set', 'values'].includes(name)) {
			return args.some((arg) => DrizzleQueryPass.#isComplexArgument(arg, imports));
		}

		if (['as', 'except', 'groupBy', 'intersect', 'orderBy', 'union', 'unionAll'].includes(name)) {
			return args.length > 1 || args.some((arg) => DrizzleQueryPass.#isStructuralArgument(arg, imports));
		}

		return args.length > 1 && args.some((arg) => DrizzleQueryPass.#isComplexArgument(arg, imports));
	}

	// Emission: render recognised structures and produce non-overlapping edits.

	static #formatArrayExpression(source: string, node: Node, imports: DrizzleImports, parsed: ParsedSourceDto, indent: string, indentUnit: string): string {
		if (parsed.hasCommentBetween(DrizzleQueryPass.#ast.getStart(node), DrizzleQueryPass.#ast.getEnd(node))) {
			return DrizzleQueryPass.#ast.sourceOf(source, node);
		}

		const elements = Array.isArray(node.elements) ? node.elements : [];

		if (elements.length === 0) {
			return '[]';
		}

		const nextIndent = `${indent}${indentUnit}`;

		const formatted = elements.map((element) => {
			return element instanceof Node ? DrizzleQueryPass.#formatNode(source, element, imports, parsed, nextIndent, indentUnit) : '';
		});

		return `[\n${nextIndent}${formatted.join(`,\n${nextIndent}`)},\n${indent}]`;
	}

	static #formatObjectExpression(source: string, node: Node, imports: DrizzleImports, parsed: ParsedSourceDto, indent: string, indentUnit: string): string {
		if (parsed.hasCommentBetween(DrizzleQueryPass.#ast.getStart(node), DrizzleQueryPass.#ast.getEnd(node))) {
			return DrizzleQueryPass.#ast.sourceOf(source, node);
		}

		const properties = DrizzleQueryPass.#ast.childNodes(node, 'properties');

		if (properties.length === 0) {
			return '{}';
		}

		const nextIndent = `${indent}${indentUnit}`;

		const formatted = properties.map((property) => {
			if (property.type !== 'Property') {
				return DrizzleQueryPass.#ast.sourceOf(source, property);
			}

			const key = DrizzleQueryPass.#ast.childNode(property, 'key');
			const value = DrizzleQueryPass.#ast.childNode(property, 'value');

			if (!key || !value || property.computed || property.method) {
				return DrizzleQueryPass.#ast.sourceOf(source, property);
			}

			if (property.shorthand) {
				return DrizzleQueryPass.#ast.sourceOf(source, property);
			}

			return `${DrizzleQueryPass.#ast.sourceOf(source, key)}: ${DrizzleQueryPass.#formatNode(source, value, imports, parsed, nextIndent, indentUnit)}`;
		});

		return `{\n${nextIndent}${formatted.join(`,\n${nextIndent}`)},\n${indent}}`;
	}

	static #formatHelperCall(source: string, call: Node, imports: DrizzleImports, parsed: ParsedSourceDto, indent: string, indentUnit: string): string {
		if (parsed.hasCommentBetween(DrizzleQueryPass.#ast.getStart(call), DrizzleQueryPass.#ast.getEnd(call))) {
			return DrizzleQueryPass.#ast.sourceOf(source, call);
		}

		const importedName = DrizzleQueryPass.#calleeName(DrizzleQueryPass.#ast.unwrapChainExpression(DrizzleQueryPass.#ast.childNode(call, 'callee')), imports);

		const args = DrizzleQueryPass.#ast.childNodes(call, 'arguments');

		if (!importedName || !MULTILINE_HELPERS.has(importedName) || args.length === 0) {
			return DrizzleQueryPass.#ast.sourceOf(source, call);
		}

		const nextIndent = `${indent}${indentUnit}`;
		const formatted = args.map((arg) => DrizzleQueryPass.#formatNode(source, arg, imports, parsed, nextIndent, indentUnit));

		return `${DrizzleQueryPass.#callDisplayName(source, call, imports)}(\n${nextIndent}${formatted.join(`,\n${nextIndent}`)},\n${indent})`;
	}

	static #formatSetOperationCall(source: string, call: Node, imports: DrizzleImports, parsed: ParsedSourceDto, indent: string, indentUnit: string): string {
		if (parsed.hasCommentBetween(DrizzleQueryPass.#ast.getStart(call), DrizzleQueryPass.#ast.getEnd(call))) {
			return DrizzleQueryPass.#ast.sourceOf(source, call);
		}

		const args = DrizzleQueryPass.#ast.childNodes(call, 'arguments');

		if (args.length < 2) {
			return DrizzleQueryPass.#ast.sourceOf(source, call);
		}

		const nextIndent = `${indent}${indentUnit}`;
		const formatted = args.map((arg) => DrizzleQueryPass.#formatNode(source, arg, imports, parsed, nextIndent, indentUnit));

		return `${DrizzleQueryPass.#callDisplayName(source, call, imports)}(\n${nextIndent}${formatted.join(`,\n${nextIndent}`)},\n${indent})`;
	}

	static #formatNode(source: string, node: Node, imports: DrizzleImports, parsed: ParsedSourceDto, indent: string, indentUnit: string): string {
		if (node.type === 'ObjectExpression' && DrizzleQueryPass.#shouldFormatObjectExpression(node)) {
			return DrizzleQueryPass.#formatObjectExpression(source, node, imports, parsed, indent, indentUnit);
		}

		if (node.type === 'ArrayExpression' && DrizzleQueryPass.#shouldFormatArrayExpression(node)) {
			return DrizzleQueryPass.#formatArrayExpression(source, node, imports, parsed, indent, indentUnit);
		}

		if (node.type === 'CallExpression') {
			if (DrizzleQueryPass.#isSetOperationCall(node, imports)) {
				return DrizzleQueryPass.#formatSetOperationCall(source, node, imports, parsed, indent, indentUnit);
			}

			if (DrizzleQueryPass.#isImportedHelperCall(node, imports)) {
				return DrizzleQueryPass.#formatHelperCall(source, node, imports, parsed, indent, indentUnit);
			}
		}

		return DrizzleQueryPass.#ast.sourceOf(source, node);
	}

	static #formatCallArguments(document: SourceDocument, call: Node, imports: DrizzleImports, parsed: ParsedSourceDto, indentUnit: string): Edit | null {
		const parens = DrizzleQueryPass.#callParens(document.text, call);
		const args = DrizzleQueryPass.#ast.childNodes(call, 'arguments');

		if (!parens || args.length === 0) {
			return null;
		}

		if (parsed.hasCommentBetween(parens.open, parens.close)) {
			return null;
		}

		const callee = DrizzleQueryPass.#ast.unwrapChainExpression(DrizzleQueryPass.#ast.childNode(call, 'callee'));

		const property = callee ? DrizzleQueryPass.#ast.childNode(callee, 'property') : undefined;
		const indentPos = callee?.type === 'MemberExpression' && property ? DrizzleQueryPass.#ast.getStart(property) : DrizzleQueryPass.#ast.getStart(call);
		const indent = document.lineIndent(indentPos);
		const argIndent = `${indent}${indentUnit}`;
		const formatted = args.map((arg) => DrizzleQueryPass.#formatNode(document.text, arg, imports, parsed, argIndent, indentUnit));
		const replacement = `(\n${argIndent}${formatted.join(`,\n${argIndent}`)},\n${indent})`;

		if (document.slice(parens.open, parens.close + 1) === replacement) {
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
	 * @param document - The document to inspect.
	 * @returns Non-overlapping query-formatting edits, or none for invalid source.
	 */
	computeEdits(document: SourceDocument): Edit[] {
		if (FileTargets.isDeclarationFile(document.virtualName)) {
			return [];
		}

		const parsed = this.#parser.parse(document.virtualName, document.text);

		if (isErr(parsed)) {
			return [];
		}

		const imports = DrizzleQueryPass.#collectDrizzleImports(parsed.value.program);

		if (imports.locals.size === 0 && imports.namespaces.size === 0) {
			return [];
		}

		const edits: Edit[] = [];
		const indentUnit = document.indentUnit();

		DrizzleQueryPass.#ast.visit(parsed.value.program, (node) => {
			if (node.type !== 'CallExpression') {
				return;
			}

			if (DrizzleQueryPass.#isDrizzleMethodCall(node, imports) || DrizzleQueryPass.#isRelationalQueryCall(node, imports) || DrizzleQueryPass.#isSetOperationCall(node, imports)) {
				const args = DrizzleQueryPass.#ast.childNodes(node, 'arguments');

				if (DrizzleQueryPass.#isSetOperationCall(node, imports) && args.length > 0 && args.length < 2) {
					return;
				}

				if (!DrizzleQueryPass.#isSetOperationCall(node, imports) && !DrizzleQueryPass.#shouldFormatMethodArguments(node, imports)) {
					return;
				}

				const edit = DrizzleQueryPass.#formatCallArguments(document, node, imports, parsed.value, indentUnit);

				if (edit) {
					edits.push(edit);
				}
			}
		});

		return this.#edits.nonOverlapping(edits);
	}
}
