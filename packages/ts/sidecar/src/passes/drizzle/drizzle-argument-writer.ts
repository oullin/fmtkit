import type { AstReader } from '#sidecar/syntax/ast-reader';
import type { DrizzleCallClassifier } from '#sidecar/passes/drizzle/drizzle-call-classifier';
import type { DrizzleImports } from '#sidecar/passes/drizzle/drizzle-import-scanner';
import type { DrizzleVocabulary } from '#sidecar/passes/drizzle/drizzle-vocabulary';
import { Node } from '#sidecar/syntax/node-schema';
import type { ParsedSourceDto } from '#sidecar/syntax/node-schema';
import type { Edit } from '#sidecar/syntax/edits';
import type { SourceDocument } from '#sidecar/syntax/source-document';

/**
 * Renders recognised Drizzle structures into stable multiline layouts.
 *
 * The writer owns the emission half of the pass: given a call the
 * {@link DrizzleCallClassifier} has approved, it produces the single
 * argument-parenthesis {@link Edit} that expands the call, recursing through
 * object, array, helper, and set-operation operands. Commented spans are left
 * verbatim so no edit rewrites source a reader annotated.
 */
export class DrizzleArgumentWriter {
	readonly #ast: AstReader;
	readonly #vocabulary: DrizzleVocabulary;
	readonly #classifier: DrizzleCallClassifier;

	/**
	 * @param dependencies - The services consumed by the writer.
	 * @param dependencies.ast - Traverses and reads validated node fields.
	 * @param dependencies.vocabulary - The recognised Drizzle name vocabulary.
	 * @param dependencies.classifier - Decides which structures may be formatted.
	 */
	constructor(dependencies: { ast: AstReader; vocabulary: DrizzleVocabulary; classifier: DrizzleCallClassifier }) {
		this.#ast = dependencies.ast;
		this.#vocabulary = dependencies.vocabulary;
		this.#classifier = dependencies.classifier;
	}

	/**
	 * Produce the edit that expands a recognised call's arguments across lines.
	 *
	 * @param document - The document the call belongs to.
	 * @param call - The approved call expression.
	 * @param imports - The Drizzle imports in scope.
	 * @param parsed - The parsed source, consulted for comment spans.
	 * @param indentUnit - The document's per-level indentation unit.
	 * @returns The argument-parenthesis edit, or `null` when none is warranted.
	 */
	formatCall(document: SourceDocument, call: Node, imports: DrizzleImports, parsed: ParsedSourceDto, indentUnit: string): Edit | null {
		const parens = this.#callParens(document.text, call);
		const args = this.#ast.childNodes(call, 'arguments');

		if (!parens || args.length === 0) {
			return null;
		}

		if (parsed.hasCommentBetween(parens.open, parens.close)) {
			return null;
		}

		const callee = this.#ast.unwrapChainExpression(this.#ast.childNode(call, 'callee'));

		const property = callee ? this.#ast.childNode(callee, 'property') : undefined;
		const indentPos = callee?.type === 'MemberExpression' && property ? this.#ast.getStart(property) : this.#ast.getStart(call);
		const indent = document.lineIndent(indentPos);
		const argIndent = `${indent}${indentUnit}`;
		const formatted = args.map((arg) => this.#formatNode(document.text, arg, imports, parsed, argIndent, indentUnit));
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

	#callDisplayName(source: string, call: Node, imports: DrizzleImports): string {
		const callee = this.#ast.unwrapChainExpression(this.#ast.childNode(call, 'callee'));

		if (callee?.type === 'Identifier') {
			return this.#ast.sourceOf(source, callee);
		}

		if (callee?.type === 'MemberExpression') {
			const name = this.#classifier.calleeName(callee, imports);

			if (name) {
				return this.#ast.sourceOf(source, callee);
			}
		}

		return callee ? this.#ast.sourceOf(source, callee) : '';
	}

	#callParens(source: string, call: Node): { open: number; close: number } | null {
		return this.#ast.callParens(source, call, this.#ast.unwrapChainExpression(this.#ast.childNode(call, 'callee')));
	}

	#formatArrayExpression(source: string, node: Node, imports: DrizzleImports, parsed: ParsedSourceDto, indent: string, indentUnit: string): string {
		if (parsed.hasCommentBetween(this.#ast.getStart(node), this.#ast.getEnd(node))) {
			return this.#ast.sourceOf(source, node);
		}

		const elements = Array.isArray(node.elements) ? node.elements : [];

		if (elements.length === 0) {
			return '[]';
		}

		const nextIndent = `${indent}${indentUnit}`;

		const formatted = elements.map((element) => {
			return element instanceof Node ? this.#formatNode(source, element, imports, parsed, nextIndent, indentUnit) : '';
		});

		return `[\n${nextIndent}${formatted.join(`,\n${nextIndent}`)},\n${indent}]`;
	}

	#formatObjectExpression(source: string, node: Node, imports: DrizzleImports, parsed: ParsedSourceDto, indent: string, indentUnit: string): string {
		if (parsed.hasCommentBetween(this.#ast.getStart(node), this.#ast.getEnd(node))) {
			return this.#ast.sourceOf(source, node);
		}

		const properties = this.#ast.childNodes(node, 'properties');

		if (properties.length === 0) {
			return '{}';
		}

		const nextIndent = `${indent}${indentUnit}`;

		const formatted = properties.map((property) => {
			if (property.type !== 'Property') {
				return this.#ast.sourceOf(source, property);
			}

			const key = this.#ast.childNode(property, 'key');
			const value = this.#ast.childNode(property, 'value');

			if (!key || !value || property.computed || property.method) {
				return this.#ast.sourceOf(source, property);
			}

			if (property.shorthand) {
				return this.#ast.sourceOf(source, property);
			}

			return `${this.#ast.sourceOf(source, key)}: ${this.#formatNode(source, value, imports, parsed, nextIndent, indentUnit)}`;
		});

		return `{\n${nextIndent}${formatted.join(`,\n${nextIndent}`)},\n${indent}}`;
	}

	#formatHelperCall(source: string, call: Node, imports: DrizzleImports, parsed: ParsedSourceDto, indent: string, indentUnit: string): string {
		if (parsed.hasCommentBetween(this.#ast.getStart(call), this.#ast.getEnd(call))) {
			return this.#ast.sourceOf(source, call);
		}

		const importedName = this.#classifier.calleeName(this.#ast.unwrapChainExpression(this.#ast.childNode(call, 'callee')), imports);

		const args = this.#ast.childNodes(call, 'arguments');

		if (!importedName || !this.#vocabulary.isMultilineHelper(importedName) || args.length === 0) {
			return this.#ast.sourceOf(source, call);
		}

		const nextIndent = `${indent}${indentUnit}`;
		const formatted = args.map((arg) => this.#formatNode(source, arg, imports, parsed, nextIndent, indentUnit));

		return `${this.#callDisplayName(source, call, imports)}(\n${nextIndent}${formatted.join(`,\n${nextIndent}`)},\n${indent})`;
	}

	#formatSetOperationCall(source: string, call: Node, imports: DrizzleImports, parsed: ParsedSourceDto, indent: string, indentUnit: string): string {
		if (parsed.hasCommentBetween(this.#ast.getStart(call), this.#ast.getEnd(call))) {
			return this.#ast.sourceOf(source, call);
		}

		const args = this.#ast.childNodes(call, 'arguments');

		if (args.length < 2) {
			return this.#ast.sourceOf(source, call);
		}

		const nextIndent = `${indent}${indentUnit}`;
		const formatted = args.map((arg) => this.#formatNode(source, arg, imports, parsed, nextIndent, indentUnit));

		return `${this.#callDisplayName(source, call, imports)}(\n${nextIndent}${formatted.join(`,\n${nextIndent}`)},\n${indent})`;
	}

	#formatNode(source: string, node: Node, imports: DrizzleImports, parsed: ParsedSourceDto, indent: string, indentUnit: string): string {
		if (node.type === 'ObjectExpression' && this.#classifier.shouldFormatObjectExpression(node)) {
			return this.#formatObjectExpression(source, node, imports, parsed, indent, indentUnit);
		}

		if (node.type === 'ArrayExpression' && this.#classifier.shouldFormatArrayExpression(node)) {
			return this.#formatArrayExpression(source, node, imports, parsed, indent, indentUnit);
		}

		if (node.type === 'CallExpression') {
			if (this.#classifier.isSetOperationCall(node, imports)) {
				return this.#formatSetOperationCall(source, node, imports, parsed, indent, indentUnit);
			}

			if (this.#classifier.isImportedHelperCall(node, imports)) {
				return this.#formatHelperCall(source, node, imports, parsed, indent, indentUnit);
			}
		}

		return this.#ast.sourceOf(source, node);
	}
}
