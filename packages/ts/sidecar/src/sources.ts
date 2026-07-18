import { parseSync } from 'oxc-parser';
import { isNode } from '#sidecar/ast';
import { SourceUnparsable } from '#sidecar/errors';
import { err, ok } from '#sidecar/result';
import type { Result } from '#sidecar/result';
import type { Node } from '#sidecar/types';

/** The syntax tree and comments produced for valid source text. */
export type ParseResult = {
	/** The parsed program root. */
	readonly program: Node;

	/** The parsed comments represented as traversable nodes. */
	readonly comments: Node[];
};

/** Parses source text without exposing a broken syntax tree to formatting passes. */
export class Sources {
	/**
	 * Parse source text into the structures used by formatting passes.
	 *
	 * @param virtualName - The filename used to select parser syntax and label errors.
	 * @param content - The source text to parse.
	 * @returns The trustworthy syntax tree, or the parser diagnostics as a value.
	 */
	static parse(virtualName: string, content: string): Result<ParseResult, SourceUnparsable> {
		const parsed = parseSync(virtualName, content);

		if (parsed.errors.length > 0) {
			return err(new SourceUnparsable(virtualName, parsed.errors));
		}

		const program: unknown = parsed.program;

		if (!isNode(program)) {
			return err(new SourceUnparsable(virtualName, []));
		}

		const comments: unknown[] = Array.isArray(parsed.comments) ? parsed.comments : [];

		return ok(
			{ program, comments: comments.filter(isNode) },
		);
	}
}
