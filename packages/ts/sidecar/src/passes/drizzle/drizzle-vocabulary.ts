/**
 * The recognised Drizzle vocabulary: the method, helper, and key names that
 * decide which calls and structures the query formatter is allowed to touch.
 *
 * The name sets live here as private readonly fields behind intent-revealing
 * predicates so no other collaborator carries a bare `Set` of Drizzle words.
 * {@link DrizzleVocabulary.standard} builds the canonical vocabulary the pass
 * ships with.
 */
export class DrizzleVocabulary {
	readonly #receivers: ReadonlySet<string>;
	readonly #chainMethods: ReadonlySet<string>;
	readonly #formatMethods: ReadonlySet<string>;
	readonly #helpers: ReadonlySet<string>;
	readonly #multilineHelpers: ReadonlySet<string>;
	readonly #setOperations: ReadonlySet<string>;
	readonly #objectKeys: ReadonlySet<string>;

	/**
	 * @param vocabulary - The recognised Drizzle name sets.
	 * @param vocabulary.receivers - The conventional query-builder receiver names.
	 * @param vocabulary.chainMethods - The chainable query-builder method names.
	 * @param vocabulary.formatMethods - The methods whose arguments may be formatted.
	 * @param vocabulary.helpers - The imported condition and expression helpers.
	 * @param vocabulary.multilineHelpers - The helpers expanded across lines.
	 * @param vocabulary.setOperations - The set-operation helper names.
	 * @param vocabulary.objectKeys - The option-object keys whose values are formatted.
	 */
	constructor(vocabulary: {
		receivers: Iterable<string>;
		chainMethods: Iterable<string>;
		formatMethods: Iterable<string>;
		helpers: Iterable<string>;
		multilineHelpers: Iterable<string>;
		setOperations: Iterable<string>;
		objectKeys: Iterable<string>;
	}) {
		this.#receivers = new Set(vocabulary.receivers);
		this.#chainMethods = new Set(vocabulary.chainMethods);
		this.#formatMethods = new Set(vocabulary.formatMethods);
		this.#helpers = new Set(vocabulary.helpers);
		this.#multilineHelpers = new Set(vocabulary.multilineHelpers);
		this.#setOperations = new Set(vocabulary.setOperations);
		this.#objectKeys = new Set(vocabulary.objectKeys);

		Object.freeze(this);
	}

	/**
	 * Build the canonical Drizzle vocabulary the query pass ships with.
	 *
	 * @returns The standard vocabulary over the recognised Drizzle names.
	 */
	static standard(): DrizzleVocabulary {
		return new DrizzleVocabulary({
			receivers: ['db', 'tx'],
			chainMethods: [
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
			],
			formatMethods: [
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
			],
			helpers: [
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
			],
			multilineHelpers: ['and', 'or', 'not', 'exists', 'notExists'],
			setOperations: ['except', 'intersect', 'union', 'unionAll'],
			objectKeys: ['columns', 'extras', 'limit', 'offset', 'onUpdate', 'orderBy', 'set', 'target', 'targetWhere', 'where', 'with'],
		});
	}

	/**
	 * Report whether a name is a conventional Drizzle query-builder receiver.
	 *
	 * @param name - The identifier to test.
	 * @returns `true` when the name is a recognised receiver.
	 */
	isConventionalReceiver(name: string): boolean {
		return this.#receivers.has(name);
	}

	/**
	 * Report whether a method name is a chainable query-builder method.
	 *
	 * @param name - The method name to test.
	 * @returns `true` when the method participates in a query chain.
	 */
	isChainMethod(name: string): boolean {
		return this.#chainMethods.has(name);
	}

	/**
	 * Report whether a method name is one whose arguments may be formatted.
	 *
	 * @param name - The method name to test.
	 * @returns `true` when the method's arguments are eligible for formatting.
	 */
	isFormatMethod(name: string): boolean {
		return this.#formatMethods.has(name);
	}

	/**
	 * Report whether a name is an imported Drizzle condition or expression helper.
	 *
	 * @param name - The imported name to test.
	 * @returns `true` when the name is a recognised helper.
	 */
	isHelper(name: string): boolean {
		return this.#helpers.has(name);
	}

	/**
	 * Report whether a helper is one expanded across multiple lines.
	 *
	 * @param name - The helper name to test.
	 * @returns `true` when the helper's arguments are expanded.
	 */
	isMultilineHelper(name: string): boolean {
		return this.#multilineHelpers.has(name);
	}

	/**
	 * Report whether a name is a set-operation helper.
	 *
	 * @param name - The name to test.
	 * @returns `true` when the name is a set-operation helper.
	 */
	isSetOperation(name: string): boolean {
		return this.#setOperations.has(name);
	}

	/**
	 * Report whether an option-object key's value should be formatted.
	 *
	 * @param key - The object key to test.
	 * @returns `true` when the key introduces a structural value.
	 */
	formatsObjectKey(key: string): boolean {
		return this.#objectKeys.has(key);
	}
}
