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
	reportPass(label: string, files: readonly string[], mode: FormatMode, outcomes: PassOutcome[], failureNoun: string): boolean {
		let changedCount = 0;

		for (const outcome of outcomes) {
			if (outcome.error?._tag === 'SourceFileUnreadable' && outcome.error.isNotFound()) {
				console.warn(`[${label}] path not found, skipping: ${outcome.file}`);

				continue;
			}

			if (outcome.error) {
				console.error(outcome.error);

				return false;
			}

			if (outcome.changed) {
				changedCount++;
				console.log(`[${label}] ${mode === 'check' ? 'would change' : 'updated'} ${outcome.file}`);
			}
		}

		if (mode === 'check' && changedCount > 0) {
			console.error(`[${label}] ${changedCount} file(s) need ${failureNoun}. Run "pnpm format" to fix.`);

			return false;
		}

		console.log(`[${label}] processed ${files.length} file(s) in ${process.cwd()}, ${changedCount} ${mode === 'check' ? 'would change' : 'changed'}`);

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
	report(files: readonly string[], failures: ValidationFailure[]): boolean {
		const diagnostics: string[] = [];

		for (const failure of failures) {
			if (failure.error._tag === 'SourceFileUnreadable') {
				if (failure.error.isNotFound()) {
					console.warn(`[validate-syntax] path not found, skipping: ${failure.file}`);

					continue;
				}

				console.error(failure.error);

				return false;
			}

			for (const error of failure.error.errors) {
				diagnostics.push(this.format(failure.file, error));
			}
		}

		if (diagnostics.length > 0) {
			console.error(diagnostics.join('\n'));
			console.error(`[validate-syntax] ${diagnostics.length} syntax error(s) found after formatting.`);

			return false;
		}

		console.log(`[validate-syntax] checked ${files.length} file(s) in ${process.cwd()}`);

		return true;
	}
}
