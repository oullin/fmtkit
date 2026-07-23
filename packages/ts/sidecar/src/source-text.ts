import { Ast } from '#sidecar/ast';
import type { Node } from '#sidecar/types';

/** The opening and closing argument-parenthesis offsets of a call. */
export type CallParens = {
	/** The opening parenthesis offset. */
	readonly open: number;

	/** The closing parenthesis offset. */
	readonly close: number;
};

/** Reads source slices and offsets without mutating source text. */
export class SourceText {
	/**
	 * Find the start offset of the line containing a position.
	 *
	 * @param source - The complete source text.
	 * @param pos - A source offset.
	 * @returns The offset immediately after the preceding newline, or zero.
	 */
	static lineStart(source: string, pos: number): number {
		return source.lastIndexOf('\n', pos - 1) + 1;
	}

	/**
	 * Read the leading whitespace of the line containing a position.
	 *
	 * @param source - The complete source text.
	 * @param pos - A source offset.
	 * @returns The line's leading spaces and tabs.
	 */
	static lineIndent(source: string, pos: number): string {
		const start = SourceText.lineStart(source, pos);
		const match = source.slice(start, pos).match(/^[ \t]*/);

		return match?.[0] ?? '';
	}

	/**
	 * Infer the file's per-level indentation unit from its content.
	 *
	 * The unit is read relative to the content's baseline (minimum) indentation
	 * so that embedded blocks whose whole body sits below column zero — an HTML
	 * `<script>` body or a list-nested Markdown fence — report one nesting level
	 * rather than their absolute baseline. The baseline is the smallest leading
	 * whitespace across non-blank, non-comment-continuation lines; the unit is
	 * the first deeper line's indentation with that baseline prefix removed. This
	 * leaves column-zero source (baseline of none) behaving as a plain first-
	 * indent read. Content with no line below its baseline falls back to a tab.
	 *
	 * @param content - The source text being formatted.
	 * @returns The detected indent unit, or a tab when none can be inferred.
	 */
	static detectIndentUnit(content: string): string {
		const indents: string[] = [];

		for (const line of content.split('\n')) {
			const match = line.match(/^([ \t]*)(\S)/);

			if (!match || match[2] === '*') {
				continue;
			}

			indents.push(match[1] ?? '');
		}

		if (indents.length === 0) {
			return '\t';
		}

		const baseline = indents.reduce((shortest, indent) => {
			return indent.length < shortest.length ? indent : shortest;
		});

		for (const indent of indents) {
			if (indent.length > baseline.length) {
				return indent.slice(baseline.length);
			}
		}

		return '\t';
	}

	/**
	 * Slice the source range occupied by an AST node.
	 *
	 * @param source - The complete source text.
	 * @param node - The node whose source to read.
	 * @returns The node's source text.
	 */
	static sourceOf(source: string, node: Node): string {
		return source.slice(Ast.getStart(node), Ast.getEnd(node));
	}

	/**
	 * Report whether a complete comment lies between two offsets.
	 *
	 * @param comments - Parsed comment nodes.
	 * @param from - The inclusive lower offset.
	 * @param to - The inclusive upper offset.
	 * @returns `true` when a comment is contained by the range.
	 */
	static hasCommentBetween(comments: readonly Node[], from: number, to: number): boolean {
		return comments.some((comment) => {
			const start = Ast.getStart(comment);
			const end = Ast.getEnd(comment);

			return start >= from && end <= to;
		});
	}

	/**
	 * Locate the argument parentheses of a call with a caller-unwrapped callee.
	 *
	 * @param source - The complete source text.
	 * @param call - The call expression node.
	 * @param callee - The callee after the caller's own unwrapping rules.
	 * @returns The parenthesis offsets, or `null` when they cannot be located.
	 */
	static callParens(source: string, call: Node, callee: Node | undefined): CallParens | null {
		const calleeEnd = callee ? Ast.getEnd(callee) : -1;
		const callEnd = Ast.getEnd(call);

		if (calleeEnd < 0 || callEnd < 0) {
			return null;
		}

		const open = source.indexOf('(', calleeEnd);

		if (open < 0 || open >= callEnd) {
			return null;
		}

		const close = callEnd - 1;

		if (source[close] !== ')') {
			return null;
		}

		return { open, close };
	}

	/**
	 * Unwrap an ESTree chain expression when present.
	 *
	 * @param node - The possible chain expression.
	 * @returns Its expression child, or the original node.
	 */
	static unwrapChainExpression(node: Node | undefined): Node | undefined {
		if (node?.type === 'ChainExpression') {
			return Ast.childNode(node, 'expression');
		}

		return node;
	}
}
