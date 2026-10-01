import type { OxcErrorDto } from '#sidecar/kernel/errors';
import type { FormatMode, PassOutcome, ValidationFailure } from '#sidecar/pipeline/format-pipeline';

/** Reports formatting-pass values to the console without coupling passes to it. */
export class PassReporter {
	/**
	 * Report one formatting pass and decide whether execution may continue.
	 *
	 * @param label - The formatting pass label.
	 * @param files - The source paths requested for the pass.
	 * @param mode - Whether the pass checked or wrote source.
	 * @param outcomes - The ordered outcomes produced by the pass.
	 * @param failureNoun - The change description used in check-mode guidance.
	 * @returns `true` when no outcome or pending change makes the pass fail.
	 */
	reportPass(label: string, files: ReadonlyArray<string>, mode: FormatMode, outcomes: Array<PassOutcome>, failureNoun: string): boolean {
		let changedCount = 0;

		for (const outcome of outcomes) {
			if (outcome.error?._tag === 'SourceFileUnreadable' && outcome.error.isNotFound()) {
				process.stderr.write(`[${label}] path not found, skipping: ${outcome.file}\n`);

				continue;
			}

			if (outcome.error) {
				process.stderr.write(`${String(outcome.error)}\n`);

				return false;
			}

			if (outcome.changed) {
				changedCount++;
				process.stdout.write(`[${label}] ${mode === 'check' ? 'would change' : 'updated'} ${outcome.file}\n`);
			}
		}

		if (mode === 'check' && changedCount > 0) {
			process.stderr.write(`[${label}] ${changedCount} file(s) need ${failureNoun}. Run "pnpm format" to fix.\n`);

			return false;
		}

		process.stdout.write(`[${label}] processed ${files.length} file(s) in ${process.cwd()}, ${changedCount} ${mode === 'check' ? 'would change' : 'changed'}\n`);

		return true;
	}
}

/** Reports syntax-validation values to the console without coupling validation to it. */
export class SyntaxReporter {
	/**
	 * Format one parser diagnostic for console output.
	 *
	 * @param file - The source path associated with the diagnostic.
	 * @param error - The parser diagnostic to render.
	 * @returns A source-framed message, plain message, or stable fallback.
	 */
	format(file: string, error: OxcErrorDto): string {
		if (error.codeframe && error.codeframe.length > 0) {
			return `[validate-syntax] ${file}\n${error.codeframe.trimEnd()}`;
		}

		if (error.message && error.message.length > 0) {
			return `[validate-syntax] ${file}: ${error.message}`;
		}

		return `[validate-syntax] ${file}: syntax validation failed`;
	}

	/**
	 * Report syntax-validation failures and decide whether execution succeeded.
	 *
	 * @param files - The source paths requested for validation.
	 * @param failures - The ordered read and parse failures.
	 * @returns `true` when no reportable validation failure remains.
	 */
	report(files: ReadonlyArray<string>, failures: Array<ValidationFailure>): boolean {
		const diagnostics: Array<string> = [];

		for (const failure of failures) {
			if (failure.error._tag === 'SourceFileUnreadable') {
				if (failure.error.isNotFound()) {
					process.stderr.write(`[validate-syntax] path not found, skipping: ${failure.file}\n`);

					continue;
				}

				process.stderr.write(`${String(failure.error)}\n`);

				return false;
			}

			for (const error of failure.error.errors) {
				diagnostics.push(this.format(failure.file, error));
			}
		}

		if (diagnostics.length > 0) {
			process.stderr.write(`${diagnostics.join('\n')}\n`);
			process.stderr.write(`[validate-syntax] ${diagnostics.length} syntax error(s) found after formatting.\n`);

			return false;
		}

		process.stdout.write(`[validate-syntax] checked ${files.length} file(s) in ${process.cwd()}\n`);

		return true;
	}
}
