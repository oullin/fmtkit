import type { AstReader } from '#sidecar/syntax/ast-reader';
import type { DrizzleImports } from '#sidecar/passes/drizzle/drizzle-import-scanner';
import type { DrizzleVocabulary } from '#sidecar/passes/drizzle/drizzle-vocabulary';
import { Node } from '#sidecar/syntax/node-schema';

/**
 * Decides which calls and arguments the Drizzle query formatter may touch.
 *
 * Every predicate reads the scan's {@link DrizzleImports} so aliased helpers and
 * namespace calls resolve to their recognised names, and consults the shared
 * {@link DrizzleVocabulary} for the method, helper, and key words that gate
 * formatting. It proposes no edits: it only answers whether a node qualifies.
 */
export class DrizzleCallClassifier {
	readonly #ast: AstReader;
	readonly #vocabulary: DrizzleVocabulary;

	/**
	 * @param dependencies - The services consumed by the classifier.
	 * @param dependencies.ast - Traverses and reads validated node fields.
	 * @param dependencies.vocabulary - The recognised Drizzle name vocabulary.
	 */
	constructor(dependencies: { ast: AstReader; vocabulary: DrizzleVocabulary }) {
		this.#ast = dependencies.ast;
		this.#vocabulary = dependencies.vocabulary;
	}

	/**
	 * Resolve a callee to the exported Drizzle name it invokes, if any.
	 *
	 * @param callee - The unwrapped callee node.
	 * @param imports - The Drizzle imports in scope.
	 * @returns The recognised exported name, or `null`.
	 */
	calleeName(callee: Node | undefined, imports: DrizzleImports): string | null {
		if (!callee) {
			return null;
		}

		if (callee.type === 'Identifier') {
			const name = this.#localName(callee);

			return name ? (imports.localImport(name) ?? null) : null;
		}

		if (callee.type === 'MemberExpression' && !callee.computed) {
			const object = this.#ast.childNode(callee, 'object');

			const property = this.#localName(this.#ast.childNode(callee, 'property'));

			const objectName = this.#localName(object);

			if (objectName && property && imports.hasNamespace(objectName)) {
				return property;
			}
		}

		return null;
	}

	/**
	 * Report whether a call is a formattable Drizzle query-builder method call.
	 *
	 * @param call - The call expression to inspect.
	 * @param imports - The Drizzle imports in scope.
	 * @returns `true` when the call is a recognised builder method on a receiver.
	 */
	isDrizzleMethodCall(call: Node, imports: DrizzleImports): boolean {
		const callee = this.#ast.unwrapChainExpression(this.#ast.childNode(call, 'callee'));

		if (callee?.type !== 'MemberExpression') {
			return false;
		}

		const name = this.#propertyName(callee);

		if (!name || !this.#vocabulary.isFormatMethod(name)) {
			return false;
		}

		return this.#isDrizzleReceiver(this.#ast.childNode(callee, 'object'), imports);
	}

	/**
	 * Report whether a call is a relational query-builder `findMany`/`findFirst`.
	 *
	 * @param call - The call expression to inspect.
	 * @param imports - The Drizzle imports in scope.
	 * @returns `true` when the call reaches a `query` member on a receiver.
	 */
	isRelationalQueryCall(call: Node, imports: DrizzleImports): boolean {
		const name = this.#methodName(call);

		const callee = this.#ast.unwrapChainExpression(this.#ast.childNode(call, 'callee'));

		if ((name !== 'findMany' && name !== 'findFirst') || callee?.type !== 'MemberExpression') {
			return false;
		}

		const object = this.#ast.childNode(callee, 'object');

		return this.#chainHasQueryMember(object) && this.#isDrizzleReceiver(object, imports);
	}

	/**
	 * Report whether a call invokes an imported Drizzle helper.
	 *
	 * @param call - The call expression to inspect.
	 * @param imports - The Drizzle imports in scope.
	 * @returns `true` when the callee resolves to a recognised helper.
	 */
	isImportedHelperCall(call: Node, imports: DrizzleImports): boolean {
		const callee = this.#ast.unwrapChainExpression(this.#ast.childNode(call, 'callee'));

		const name = this.calleeName(callee, imports);

		return Boolean(name && this.#vocabulary.isHelper(name));
	}

	/**
	 * Report whether a call invokes a set-operation helper.
	 *
	 * @param call - The call expression to inspect.
	 * @param imports - The Drizzle imports in scope.
	 * @returns `true` when the callee resolves to a set-operation helper.
	 */
	isSetOperationCall(call: Node, imports: DrizzleImports): boolean {
		const callee = this.#ast.unwrapChainExpression(this.#ast.childNode(call, 'callee'));

		const name = this.calleeName(callee, imports);

		return Boolean(name && this.#vocabulary.isSetOperation(name));
	}

	/**
	 * Report whether an object expression is worth expanding across lines.
	 *
	 * @param node - The object expression to inspect.
	 * @returns `true` when the object has multiple or structural properties.
	 */
	shouldFormatObjectExpression(node: Node): boolean {
		const properties = this.#ast.childNodes(node, 'properties');

		if (properties.length > 1) {
			return true;
		}

		return properties.some((property) => {
			if (property.type !== 'Property') {
				return true;
			}

			const key = this.#localName(this.#ast.childNode(property, 'key'));

			const value = this.#ast.childNode(property, 'value');

			if (!value) {
				return false;
			}

			if (key && this.#vocabulary.formatsObjectKey(key) && (value.type === 'ObjectExpression' || value.type === 'ArrayExpression' || value.type === 'CallExpression')) {
				return true;
			}

			return value.type === 'ObjectExpression' || value.type === 'ArrayExpression';
		});
	}

	/**
	 * Report whether an array expression is worth expanding across lines.
	 *
	 * @param node - The array expression to inspect.
	 * @returns `true` when the array has multiple or structural elements.
	 */
	shouldFormatArrayExpression(node: Node): boolean {
		const elements = Array.isArray(node.elements) ? node.elements : [];

		return elements.length > 1 || elements.some((element) => element instanceof Node && (element.type === 'ObjectExpression' || element.type === 'CallExpression'));
	}

	/**
	 * Report whether a method call's arguments should be expanded.
	 *
	 * @param call - The call expression to inspect.
	 * @param imports - The Drizzle imports in scope.
	 * @returns `true` when the method's arguments qualify for formatting.
	 */
	shouldFormatMethodArguments(call: Node, imports: DrizzleImports): boolean {
		const args = this.#ast.childNodes(call, 'arguments');

		if (args.length === 0) {
			return false;
		}

		if (this.isRelationalQueryCall(call, imports)) {
			return args.some((arg) => arg.type === 'ObjectExpression' && this.shouldFormatObjectExpression(arg));
		}

		const name = this.#methodName(call);

		if (!name) {
			return false;
		}

		if (['where', 'having', '$count'].includes(name)) {
			return args.some((arg) => this.#isComplexArgument(arg, imports));
		}

		if (['leftJoin', 'rightJoin', 'innerJoin', 'fullJoin', 'crossJoin'].includes(name)) {
			return args.length > 1 && args.some((arg, index) => index > 0 && this.#isComplexArgument(arg, imports));
		}

		if (['onConflictDoNothing', 'onConflictDoUpdate', 'returning', 'set', 'values'].includes(name)) {
			return args.some((arg) => this.#isComplexArgument(arg, imports));
		}

		if (['as', 'except', 'groupBy', 'intersect', 'orderBy', 'union', 'unionAll'].includes(name)) {
			return args.length > 1 || args.some((arg) => this.#isStructuralArgument(arg, imports));
		}

		return args.length > 1 && args.some((arg) => this.#isComplexArgument(arg, imports));
	}

	#localName(node: Node | undefined): string | null {
		return node?.type === 'Identifier' ? (this.#ast.nodeName(node) ?? null) : null;
	}

	#propertyName(member: Node | undefined): string | null {
		if (member?.type !== 'MemberExpression' || member.computed) {
			return null;
		}

		return this.#localName(this.#ast.childNode(member, 'property'));
	}

	#chainHasQueryMember(node: Node | undefined): boolean {
		const current = this.#ast.unwrapChainExpression(node);

		if (!current) {
			return false;
		}

		if (current.type === 'MemberExpression') {
			if (this.#propertyName(current) === 'query') {
				return true;
			}

			return this.#chainHasQueryMember(this.#ast.childNode(current, 'object'));
		}

		if (current.type === 'CallExpression') {
			return this.#chainHasQueryMember(this.#ast.childNode(current, 'callee'));
		}

		return false;
	}

	#isDrizzleReceiver(node: Node | undefined, imports: DrizzleImports): boolean {
		const current = this.#ast.unwrapChainExpression(node);

		if (!current) {
			return false;
		}

		if (current.type === 'Identifier') {
			const name = this.#localName(current);

			return Boolean(name && this.#vocabulary.isConventionalReceiver(name));
		}

		if (current.type === 'MemberExpression') {
			const object = this.#ast.childNode(current, 'object');
			const property = this.#propertyName(current);

			if (property === 'query') {
				return this.#isDrizzleReceiver(object, imports);
			}

			return this.#isDrizzleReceiver(object, imports);
		}

		if (current.type === 'CallExpression') {
			const callee = this.#ast.unwrapChainExpression(this.#ast.childNode(current, 'callee'));

			if (callee?.type === 'Identifier') {
				const imported = this.calleeName(callee, imports);

				return Boolean(imported && this.#vocabulary.isSetOperation(imported));
			}

			if (callee?.type === 'MemberExpression') {
				const method = this.#propertyName(callee);

				if (method && this.#vocabulary.isChainMethod(method)) {
					return this.#isDrizzleReceiver(this.#ast.childNode(callee, 'object'), imports);
				}

				return this.#isDrizzleReceiver(this.#ast.childNode(callee, 'object'), imports);
			}
		}

		return false;
	}

	#methodName(call: Node): string | null {
		const callee = this.#ast.unwrapChainExpression(this.#ast.childNode(call, 'callee'));

		return callee?.type === 'MemberExpression' ? this.#propertyName(callee) : null;
	}

	#isComplexArgument(node: Node, imports: DrizzleImports): boolean {
		if (node.type === 'ObjectExpression') {
			return this.shouldFormatObjectExpression(node);
		}

		if (node.type === 'ArrayExpression') {
			return this.shouldFormatArrayExpression(node);
		}

		if (node.type === 'CallExpression') {
			return this.isImportedHelperCall(node, imports) || this.isSetOperationCall(node, imports) || this.isDrizzleMethodCall(node, imports);
		}

		return false;
	}

	#isStructuralArgument(node: Node, imports: DrizzleImports): boolean {
		if (node.type === 'ObjectExpression' || node.type === 'ArrayExpression') {
			return true;
		}

		return node.type === 'CallExpression' && (this.isSetOperationCall(node, imports) || this.isDrizzleMethodCall(node, imports));
	}
}
