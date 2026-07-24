import { Ast } from '#sidecar/syntax/ast';
import { Node } from '#sidecar/syntax/node-schema';

const BLOCK_HAVING_STATEMENTS = new Set(['IfStatement', 'ForStatement', 'ForInStatement', 'ForOfStatement', 'WhileStatement', 'DoWhileStatement', 'SwitchStatement', 'TryStatement']);
const LOOP_STATEMENTS = new Set(['ForStatement', 'ForInStatement', 'ForOfStatement', 'WhileStatement', 'DoWhileStatement']);
const TS_TYPE_DECLARATION_TYPES = new Set(['TSTypeAliasDeclaration', 'TSInterfaceDeclaration', 'TSEnumDeclaration', 'TSModuleDeclaration']);
const CLASS_METHOD_TYPES = new Set(['MethodDefinition', 'TSAbstractMethodDefinition']);
const CLASS_PROPERTY_TYPES = new Set(['PropertyDefinition', 'TSAbstractPropertyDefinition', 'AccessorProperty', 'TSIndexSignature', 'StaticBlock']);
const BLANK_LINE_ABOVE_TYPES = new Set(['SwitchStatement', 'SwitchCase', 'FunctionDeclaration', 'ClassDeclaration', 'TSEnumDeclaration', 'TSModuleDeclaration']);

const STRUCTURED_PREVIOUS_STATEMENTS = new Set([
	'ClassDeclaration',
	'DoWhileStatement',
	'ForInStatement',
	'ForOfStatement',
	'ForStatement',
	'FunctionDeclaration',
	'IfStatement',
	'SwitchStatement',
	'TryStatement',
	'WhileStatement',
]);

const VUE_PRIMITIVE_CALLS = new Set([
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

/** Encapsulates the formatter's statement and class-member layout rules. */
export class Rules {
	static #isExportWithDeclaration(node: Node): boolean {
		if (node.type !== 'ExportNamedDeclaration' && node.type !== 'ExportDefaultDeclaration') {
			return false;
		}

		return Boolean(node.declaration);
	}

	static #isBlankLineAboveType(next: Node): boolean {
		return BLANK_LINE_ABOVE_TYPES.has(next.type);
	}

	static #isIdentifierNamed(node: Node | undefined, names: Set<string>): boolean {
		if (node?.type !== 'Identifier') {
			return false;
		}

		const name = Ast.nodeName(node);

		return name !== undefined && names.has(name);
	}

	static #isVuePrimitiveCall(node: Node | undefined): boolean {
		if (node?.type !== 'CallExpression') {
			return false;
		}

		return Rules.#isIdentifierNamed(Ast.childNode(node, 'callee'), VUE_PRIMITIVE_CALLS);
	}

	static #isVuePrimitiveStatement(node: Node): boolean {
		if (node.type === 'ExpressionStatement') {
			return Rules.#isVuePrimitiveCall(Ast.childNode(node, 'expression'));
		}

		if (node.type !== 'VariableDeclaration' || Ast.declarationKind(node) !== 'const') {
			return false;
		}

		return Ast.childNodes(node, 'declarations').some((declaration) => {
			return Rules.#isVuePrimitiveCall(Ast.childNode(declaration, 'init'));
		});
	}

	static #needsBlankLineAbove(next: Node): boolean {
		if (next.type === 'ReturnStatement' || Rules.#isVuePrimitiveStatement(next) || Rules.#isBlankLineAboveType(next)) {
			return true;
		}

		return Rules.#isExportWithDeclaration(next);
	}

	static #isTypeDeclarationAbove(previous: Node): boolean {
		if (TS_TYPE_DECLARATION_TYPES.has(previous.type)) {
			return true;
		}

		if (previous.type === 'ExportNamedDeclaration') {
			const declarationType = Ast.childNode(previous, 'declaration')?.type;

			return declarationType ? TS_TYPE_DECLARATION_TYPES.has(declarationType) : false;
		}

		return false;
	}

	static #isLoopStatement(node: Node): boolean {
		return LOOP_STATEMENTS.has(node.type);
	}

	static #isStructuredPreviousStatement(previous: Node): boolean {
		if (STRUCTURED_PREVIOUS_STATEMENTS.has(previous.type)) {
			return true;
		}

		if (previous.type === 'ExportNamedDeclaration' || previous.type === 'ExportDefaultDeclaration') {
			const declarationType = Ast.childNode(previous, 'declaration')?.type;

			return Boolean(declarationType && STRUCTURED_PREVIOUS_STATEMENTS.has(declarationType));
		}

		return false;
	}

	static #isClassMethodPair(previous: Node, next: Node): boolean {
		return CLASS_METHOD_TYPES.has(previous.type) && CLASS_METHOD_TYPES.has(next.type);
	}

	static #isPropertyToMethodTransition(previous: Node, next: Node): boolean {
		return CLASS_PROPERTY_TYPES.has(previous.type) && CLASS_METHOD_TYPES.has(next.type);
	}

	static #isLetDeclaration(node: Node): boolean {
		return node.type === 'VariableDeclaration' && Ast.declarationKind(node) === 'let';
	}

	static #containsAwait(node: Node): boolean {
		if (node.type === 'AwaitExpression') {
			return true;
		}

		if (node.type === 'FunctionDeclaration' || node.type === 'FunctionExpression' || node.type === 'ArrowFunctionExpression') {
			return false;
		}

		for (const value of Object.values(node)) {
			if (Array.isArray(value)) {
				if (
					value.some((child) => {
						return child instanceof Node && Rules.#containsAwait(child);
					})
				) {
					return true;
				}
			} else if (value instanceof Node && Rules.#containsAwait(value)) {
				return true;
			}
		}

		return false;
	}

	/**
	 * Decide whether two adjacent statements require a blank line.
	 *
	 * @param previous - The previous statement.
	 * @param next - The following statement.
	 * @returns `true` when the pair must be separated by a blank line.
	 */
	static needsBlankLine(previous: Node, next: Node): boolean {
		if (Rules.#containsAwait(previous) || Rules.#containsAwait(next) || Rules.#needsBlankLineAbove(next)) {
			return true;
		}

		if (Rules.#isLoopStatement(next)) {
			return !Rules.#isStructuredPreviousStatement(previous);
		}

		if (Rules.#isClassMethodPair(previous, next) || Rules.#isPropertyToMethodTransition(previous, next) || Rules.#isTypeDeclarationAbove(previous)) {
			return true;
		}

		if (previous.type === 'ImportDeclaration' && next.type !== 'ImportDeclaration') {
			return true;
		}

		if (Ast.isConstDeclaration(previous) !== Ast.isConstDeclaration(next)) {
			return true;
		}

		if (Rules.#isLetDeclaration(previous) !== Rules.#isLetDeclaration(next)) {
			return true;
		}

		if (previous.type === 'VariableDeclaration' && next.type !== 'VariableDeclaration') {
			return true;
		}

		return BLOCK_HAVING_STATEMENTS.has(previous.type);
	}

	/**
	 * Classify a class member for stable ordering.
	 *
	 * @param node - The class member to classify.
	 * @returns Its property, constructor, or method group.
	 */
	static classifyMember(node: Node): 'property' | 'constructor' | 'method' {
		if (CLASS_PROPERTY_TYPES.has(node.type)) {
			return 'property';
		}

		if (node.type === 'MethodDefinition' && Ast.declarationKind(node) === 'constructor') {
			return 'constructor';
		}

		return 'method';
	}
}
