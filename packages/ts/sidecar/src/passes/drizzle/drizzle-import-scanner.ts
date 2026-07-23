import type { AstReader } from '#sidecar/syntax/ast-reader';
import type { Node } from '#sidecar/syntax/node-schema';

/**
 * The Drizzle imports a module brings into scope.
 *
 * A frozen value object capturing what a scan produces: a map from each named
 * import's local binding to its original exported name (so aliases resolve back
 * to the recognised helper) and the set of namespace-import bindings. Consumers
 * ask about a binding through {@link DrizzleImports.localImport} and
 * {@link DrizzleImports.hasNamespace} rather than reaching into the collections.
 */
export class DrizzleImports {
	readonly #locals: ReadonlyMap<string, string>;
	readonly #namespaces: ReadonlySet<string>;

	private constructor(locals: ReadonlyMap<string, string>, namespaces: ReadonlySet<string>) {
		this.#locals = locals;
		this.#namespaces = namespaces;

		Object.freeze(this);
	}

	/**
	 * Build imports from a scan's accumulated local and namespace bindings.
	 *
	 * @param locals - The local-binding to exported-name map.
	 * @param namespaces - The namespace-import bindings.
	 * @returns The frozen imports value object.
	 */
	static of(locals: Map<string, string>, namespaces: Set<string>): DrizzleImports {
		return new DrizzleImports(new Map(locals), new Set(namespaces));
	}

	/**
	 * Build the empty imports carrying no Drizzle bindings.
	 *
	 * @returns The frozen empty imports.
	 */
	static empty(): DrizzleImports {
		return new DrizzleImports(new Map(), new Set());
	}

	/** Whether the module imports nothing from Drizzle. */
	get isEmpty(): boolean {
		return this.#locals.size === 0 && this.#namespaces.size === 0;
	}

	/**
	 * Resolve a local binding to the exported Drizzle name it was imported as.
	 *
	 * @param local - The local identifier used at the call site.
	 * @returns The original exported name, or `undefined` when it is not imported.
	 */
	localImport(local: string): string | undefined {
		return this.#locals.get(local);
	}

	/**
	 * Report whether a name is bound to a Drizzle namespace import.
	 *
	 * @param name - The identifier to test.
	 * @returns `true` when the name is a namespace binding.
	 */
	hasNamespace(name: string): boolean {
		return this.#namespaces.has(name);
	}
}

/** Collects the Drizzle imports a module brings into scope. */
export class DrizzleImportScanner {
	readonly #ast: AstReader;

	readonly #module = 'drizzle-orm';

	/**
	 * @param dependencies - The syntax services consumed by the scanner.
	 * @param dependencies.ast - Traverses and reads validated node fields.
	 */
	constructor(dependencies: { ast: AstReader }) {
		this.#ast = dependencies.ast;
	}

	/**
	 * Collect the Drizzle imports declared at the top of a program.
	 *
	 * @param program - The parsed program root to scan.
	 * @returns The imports the module brings into scope.
	 */
	scan(program: Node): DrizzleImports {
		const locals = new Map<string, string>();
		const namespaces = new Set<string>();
		const body = this.#ast.childNodes(program, 'body');

		for (const statement of body) {
			if (statement.type !== 'ImportDeclaration') {
				continue;
			}

			const source = this.#literalValue(this.#ast.childNode(statement, 'source'));

			if (!source?.startsWith(this.#module)) {
				continue;
			}

			for (const specifier of this.#ast.childNodes(statement, 'specifiers')) {
				if (specifier.type === 'ImportSpecifier') {
					const imported = this.#localName(this.#ast.childNode(specifier, 'imported'));
					const local = this.#localName(this.#ast.childNode(specifier, 'local'));

					if (imported && local) {
						locals.set(local, imported);
					}
				}

				if (specifier.type === 'ImportNamespaceSpecifier') {
					const local = this.#localName(this.#ast.childNode(specifier, 'local'));

					if (local) {
						namespaces.add(local);
					}
				}
			}
		}

		return DrizzleImports.of(locals, namespaces);
	}

	#localName(node: Node | undefined): string | null {
		return node?.type === 'Identifier' ? (this.#ast.nodeName(node) ?? null) : null;
	}

	#literalValue(node: Node | undefined): string | null {
		if (node?.type !== 'Literal') {
			return null;
		}

		return this.#ast.stringValue(node) ?? null;
	}
}
