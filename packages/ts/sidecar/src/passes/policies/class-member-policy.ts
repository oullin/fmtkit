import type { AstReader } from '#sidecar/syntax/ast-reader';
import type { Node } from '#sidecar/syntax/node-schema';

/** The class member group used to sort a class into its stable shape. */
export type ClassMemberKind = 'property' | 'constructor' | 'method';

/** Classifies class members and the transitions that require a blank line. */
export class ClassMemberPolicy {
	readonly #ast: AstReader;

	readonly #methodTypes: ReadonlySet<string> = new Set(['MethodDefinition', 'TSAbstractMethodDefinition']);

	readonly #propertyTypes: ReadonlySet<string> = new Set(['PropertyDefinition', 'TSAbstractPropertyDefinition', 'AccessorProperty', 'TSIndexSignature', 'StaticBlock']);

	/**
	 * @param dependencies - The syntax services consumed by the policy.
	 * @param dependencies.ast - Reads validated scalar fields off trusted nodes.
	 */
	constructor(dependencies: { ast: AstReader }) {
		this.#ast = dependencies.ast;
	}

	/**
	 * Classify a class member for stable ordering.
	 *
	 * @param node - The class member to classify.
	 * @returns Its property, constructor, or method group.
	 */
	classify(node: Node): ClassMemberKind {
		if (this.#propertyTypes.has(node.type)) {
			return 'property';
		}

		if (node.type === 'MethodDefinition' && this.#ast.declarationKind(node) === 'constructor') {
			return 'constructor';
		}

		return 'method';
	}

	/**
	 * Report whether two adjacent members are both class methods.
	 *
	 * @param previous - The previous member.
	 * @param next - The following member.
	 * @returns `true` when both nodes are method definitions.
	 */
	isMethodPair(previous: Node, next: Node): boolean {
		return this.#methodTypes.has(previous.type) && this.#methodTypes.has(next.type);
	}

	/**
	 * Report whether a member transitions from a property to a method.
	 *
	 * @param previous - The previous member.
	 * @param next - The following member.
	 * @returns `true` when a property is immediately followed by a method.
	 */
	isPropertyToMethodTransition(previous: Node, next: Node): boolean {
		return this.#propertyTypes.has(previous.type) && this.#methodTypes.has(next.type);
	}
}
