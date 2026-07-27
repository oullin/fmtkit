import type { AstReader } from '#sidecar/syntax/ast-reader';
import type { Node } from '#sidecar/syntax/node-schema';

/** Recognises Vue reactivity primitives that earn a blank line above. */
export class VueReactivityIdioms {
	readonly #ast: AstReader;

	readonly #primitiveCalls: ReadonlySet<string> = new Set([
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

	/**
	 * @param dependencies - The syntax services consumed by the policy.
	 * @param dependencies.ast - Reads validated scalar fields off trusted nodes.
	 */
	constructor(dependencies: { ast: AstReader }) {
		this.#ast = dependencies.ast;
	}

	/**
	 * Report whether a statement declares or invokes a Vue reactivity primitive.
	 *
	 * @param node - The statement to inspect.
	 * @returns `true` for a primitive call expression or `const` bound to one.
	 */
	isVuePrimitiveStatement(node: Node): boolean {
		if (node.type === 'ExpressionStatement') {
			return this.#isVuePrimitiveCall(this.#ast.childNode(node, 'expression'));
		}

		if (node.type !== 'VariableDeclaration' || this.#ast.declarationKind(node) !== 'const') {
			return false;
		}

		return this.#ast.childNodes(node, 'declarations').some((declaration) => {
			return this.#isVuePrimitiveCall(this.#ast.childNode(declaration, 'init'));
		});
	}

	#isVuePrimitiveCall(node: Node | undefined): boolean {
		if (node?.type !== 'CallExpression') {
			return false;
		}

		return this.#isIdentifierNamed(this.#ast.childNode(node, 'callee'), this.#primitiveCalls);
	}

	#isIdentifierNamed(node: Node | undefined, names: ReadonlySet<string>): boolean {
		if (node?.type !== 'Identifier') {
			return false;
		}

		const name = this.#ast.nodeName(node);

		return name !== undefined && names.has(name);
	}
}
