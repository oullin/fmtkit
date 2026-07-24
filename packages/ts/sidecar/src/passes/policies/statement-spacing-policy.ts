import type { AstReader } from '#sidecar/syntax/ast-reader';
import type { ClassMemberPolicy } from '#sidecar/passes/policies/class-member-policy';
import { Node } from '#sidecar/syntax/node-schema';
import type { VueReactivityIdioms } from '#sidecar/passes/policies/vue-reactivity-idioms';

/** Decides which adjacent statements the formatter separates with a blank line. */
export class StatementSpacingPolicy {
	readonly #ast: AstReader;
	readonly #members: ClassMemberPolicy;
	readonly #vue: VueReactivityIdioms;

	readonly #blockHavingStatements: ReadonlySet<string> = new Set([
		'IfStatement',
		'ForStatement',
		'ForInStatement',
		'ForOfStatement',
		'WhileStatement',
		'DoWhileStatement',
		'SwitchStatement',
		'TryStatement',
	]);

	readonly #loopStatements: ReadonlySet<string> = new Set(['ForStatement', 'ForInStatement', 'ForOfStatement', 'WhileStatement', 'DoWhileStatement']);

	readonly #typeDeclarationTypes: ReadonlySet<string> = new Set(['TSTypeAliasDeclaration', 'TSInterfaceDeclaration', 'TSEnumDeclaration', 'TSModuleDeclaration']);

	readonly #blankLineAboveTypes: ReadonlySet<string> = new Set(['SwitchStatement', 'SwitchCase', 'FunctionDeclaration', 'ClassDeclaration', 'TSEnumDeclaration', 'TSModuleDeclaration']);

	readonly #structuredPreviousStatements: ReadonlySet<string> = new Set([
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

	/**
	 * @param dependencies - The syntax services and sibling policies consumed here.
	 * @param dependencies.ast - Reads validated scalar fields off trusted nodes.
	 * @param dependencies.members - Classifies class-member spacing transitions.
	 * @param dependencies.vue - Recognises Vue reactivity primitive statements.
	 */
	constructor(dependencies: { ast: AstReader; members: ClassMemberPolicy; vue: VueReactivityIdioms }) {
		this.#ast = dependencies.ast;
		this.#members = dependencies.members;
		this.#vue = dependencies.vue;
	}

	/**
	 * Decide whether two adjacent statements require a blank line.
	 *
	 * @param previous - The previous statement.
	 * @param next - The following statement.
	 * @returns `true` when the pair must be separated by a blank line.
	 */
	needsBlankLine(previous: Node, next: Node): boolean {
		if (this.#containsAwait(previous) || this.#containsAwait(next) || this.#needsBlankLineAbove(next)) {
			return true;
		}

		if (this.#isLoopStatement(next)) {
			return !this.#isStructuredPreviousStatement(previous);
		}

		if (this.#members.isMethodPair(previous, next) || this.#members.isPropertyToMethodTransition(previous, next) || this.#isTypeDeclarationAbove(previous)) {
			return true;
		}

		if (previous.type === 'ImportDeclaration' && next.type !== 'ImportDeclaration') {
			return true;
		}

		if (this.#ast.isConstDeclaration(previous) !== this.#ast.isConstDeclaration(next)) {
			return true;
		}

		if (this.#isLetDeclaration(previous) !== this.#isLetDeclaration(next)) {
			return true;
		}

		if (previous.type === 'VariableDeclaration' && next.type !== 'VariableDeclaration') {
			return true;
		}

		return this.#blockHavingStatements.has(previous.type);
	}

	#isExportWithDeclaration(node: Node): boolean {
		if (node.type !== 'ExportNamedDeclaration' && node.type !== 'ExportDefaultDeclaration') {
			return false;
		}

		return Boolean(node.declaration);
	}

	#isBlankLineAboveType(next: Node): boolean {
		return this.#blankLineAboveTypes.has(next.type);
	}

	#needsBlankLineAbove(next: Node): boolean {
		if (next.type === 'ReturnStatement' || this.#vue.isVuePrimitiveStatement(next) || this.#isBlankLineAboveType(next)) {
			return true;
		}

		return this.#isExportWithDeclaration(next);
	}

	#isTypeDeclarationAbove(previous: Node): boolean {
		if (this.#typeDeclarationTypes.has(previous.type)) {
			return true;
		}

		if (previous.type === 'ExportNamedDeclaration') {
			const declarationType = this.#ast.childNode(previous, 'declaration')?.type;

			return declarationType ? this.#typeDeclarationTypes.has(declarationType) : false;
		}

		return false;
	}

	#isLoopStatement(node: Node): boolean {
		return this.#loopStatements.has(node.type);
	}

	#isStructuredPreviousStatement(previous: Node): boolean {
		if (this.#structuredPreviousStatements.has(previous.type)) {
			return true;
		}

		if (previous.type === 'ExportNamedDeclaration' || previous.type === 'ExportDefaultDeclaration') {
			const declarationType = this.#ast.childNode(previous, 'declaration')?.type;

			return Boolean(declarationType && this.#structuredPreviousStatements.has(declarationType));
		}

		return false;
	}

	#isLetDeclaration(node: Node): boolean {
		return node.type === 'VariableDeclaration' && this.#ast.declarationKind(node) === 'let';
	}

	#containsAwait(node: Node): boolean {
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
						return child instanceof Node && this.#containsAwait(child);
					})
				) {
					return true;
				}
			} else if (value instanceof Node && this.#containsAwait(value)) {
				return true;
			}
		}

		return false;
	}
}
