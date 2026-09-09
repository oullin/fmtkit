import { childEntries, childNode, isFunctionNode, stringProperty } from '#sidecar/complexity/ast-entries';
import type { Node } from '#sidecar/syntax/node-schema';

/** The reporting name given to a function no declaration names. */
export const ANONYMOUS = '<anonymous>';

/** One function-like node found in a file, with the key name it reports under. */
export type FunctionSite = {
	/** The function, arrow, or function-expression node. */
	readonly node: Node;

	/** The reporting name: this function's own, or its nearest named ancestor's. */
	readonly name: string;

	/** Whether the name belongs to this node rather than an ancestor. */
	readonly named: boolean;

	/** The source offset the function starts at. */
	readonly start: number;
};

/** The naming context a node inherits from the declaration that encloses it. */
type NameContext = {
	/** The name a function sitting at this exact position would take. */
	readonly pending: string;

	/** The nearest named function ancestor's reporting name. */
	readonly owner: string;

	/** The enclosing class name, used to qualify its members. */
	readonly className: string;
};

/** Where a declaration keeps the function it names, and where it keeps the name. */
type NameSource = {
	/** The property holding the function. */
	readonly holds: string;

	/** The property holding the name. */
	readonly names: string;

	/** Whether the enclosing class name qualifies the result. */
	readonly qualified: boolean;
};

const NAME_SOURCES = new Map<string, NameSource>([
	['AssignmentExpression', { holds: 'right', names: 'left', qualified: false }],
	['MethodDefinition', { holds: 'value', names: 'key', qualified: true }],
	['Property', { holds: 'value', names: 'key', qualified: false }],
	['PropertyDefinition', { holds: 'value', names: 'key', qualified: true }],
	['VariableDeclarator', { holds: 'init', names: 'id', qualified: false }],
]);

const CLASS_TYPES: ReadonlySet<string> = new Set(['ClassDeclaration', 'ClassExpression']);

const ACCESSOR_KINDS: ReadonlySet<string> = new Set(['get', 'set']);

/**
 * Render the `get `/`set ` prefix an accessor reports under. A getter and a
 * setter share one property name, so without it the pair would collide on a
 * single key and one of them would silently inherit the other's allow entry.
 *
 * @param owner - The member declaration the function hangs off.
 * @returns `'get '`, `'set '`, or `''` for every other member.
 */
function accessorPrefix(owner: Node): string {
	const kind = stringProperty(owner, 'kind');

	return ACCESSOR_KINDS.has(kind) ? `${kind} ` : '';
}

/**
 * Render the name a node writes: an identifier, a literal key, a private
 * name, or a dotted member path.
 *
 * @param node - The naming node, when there is one.
 * @returns The rendered name, or `''` when the node names nothing usable.
 */
function nameText(node: Node | undefined): string {
	if (!node) {
		return '';
	}

	if (node.type === 'MemberExpression') {
		return [nameText(childNode(node, 'object')), nameText(childNode(node, 'property'))].filter(Boolean).join('.');
	}

	if (node.type === 'PrivateIdentifier') {
		return `#${stringProperty(node, 'name')}`;
	}

	return stringProperty(node, 'name') || stringProperty(node, 'value');
}

/**
 * Walks a program and reports every function-like node with the name it is
 * accountable to.
 *
 * A function that a declaration names reports under that name. One that
 * nothing names — a callback, an inline handler — reports under the nearest
 * named function above it, so a closure's cost lands on the declaration a
 * reader would go and open.
 */
export class FunctionSiteCollector {
	/**
	 * Collect the function sites of a parsed program in source order.
	 *
	 * @param program - The parsed program root.
	 * @returns Every function-like node, outermost first.
	 */
	collect(program: Node): FunctionSite[] {
		const sites: FunctionSite[] = [];

		this.#visit(program, { pending: '', owner: '', className: '' }, sites);

		return sites;
	}

	#visit(node: Node, context: NameContext, sites: FunctionSite[]): void {
		if (isFunctionNode(node)) {
			this.#visitFunction(node, context, sites);

			return;
		}

		const className = CLASS_TYPES.has(node.type) ? nameText(childNode(node, 'id')) || context.pending : context.className;

		this.#descend(node, { ...context, className }, sites);
	}

	#visitFunction(node: Node, context: NameContext, sites: FunctionSite[]): void {
		const own = nameText(childNode(node, 'id')) || context.pending;
		const name = own || context.owner || ANONYMOUS;

		sites.push({ node, name, named: own !== '', start: node.start ?? node.range?.[0] ?? 0 });

		this.#descend(node, { pending: '', owner: name, className: context.className }, sites);
	}

	#descend(node: Node, context: NameContext, sites: FunctionSite[]): void {
		for (const entry of childEntries(node)) {
			this.#visit(entry.child, { ...context, pending: this.#pendingFor(node, entry.key, context.className) }, sites);
		}
	}

	#pendingFor(parent: Node, key: string, className: string): string {
		if (parent.type === 'ExportDefaultDeclaration') {
			return 'default';
		}

		const source = NAME_SOURCES.get(parent.type);

		if (!source || source.holds !== key) {
			return '';
		}

		const name = accessorPrefix(parent) + nameText(childNode(parent, source.names));

		if (name === '' || !source.qualified || !className) {
			return name;
		}

		return `${className}.${name}`;
	}
}
