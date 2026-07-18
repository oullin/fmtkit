import { parseSync } from 'oxc-parser';
import { SourceUnparsable } from '#sidecar/errors';
import { ParsedSourceDto } from '#sidecar/node-schema';
import { err, ok } from '#sidecar/result';
import type { Result } from '#sidecar/result';

/** Parses source text without exposing a broken syntax tree to formatting passes. */
export class Sources {
	/**
	 * Parse source text into the structures used by formatting passes.
	 *
	 * @param virtualName - The filename used to select parser syntax and label errors.
	 * @param content - The source text to parse.
	 * @returns The trustworthy syntax tree, or the parser diagnostics as a value.
	 */
	static parse(virtualName: string, content: string): Result<ParsedSourceDto, SourceUnparsable> {
		const parsed = parseSync(virtualName, content);

		if (parsed.errors.length > 0) {
			return err(new SourceUnparsable(virtualName, parsed.errors));
		}

		const validated = ParsedSourceDto.from({ program: parsed.program, comments: parsed.comments });

		if (!validated.success) {
			return err(new SourceUnparsable(virtualName, []));
		}

		return ok(validated.data);
	}
}
