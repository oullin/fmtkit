import { CognitiveScorer } from '#sidecar/complexity/cognitive-scorer';
import { CyclomaticScorer } from '#sidecar/complexity/cyclomatic-scorer';
import { FunctionSiteCollector } from '#sidecar/complexity/function-names';
import type { FunctionSite } from '#sidecar/complexity/function-names';
import { LineIndex } from '#sidecar/complexity/line-index';
import { SourceParser } from '#sidecar/syntax/source-parser';
import { isErr } from '#sidecar/kernel/result';

/** One scored function, keyed the way the allow list and the findings key it. */
export type ScoredFunction = {
	/** SonarSource-shaped cognitive complexity. */
	readonly cognitive: number;

	/** ESLint-shaped cyclomatic complexity. */
	readonly cyclomatic: number;

	/** The repository-relative file. */
	readonly file: string;

	/** The `<file>#<name>` key. */
	readonly key: string;

	/** The one-based line the function starts on. */
	readonly line: number;

	/** The reporting name within the file. */
	readonly name: string;
};

/** A file the scan could not read or parse. */
export type ScanFailure = {
	/** The repository-relative file. */
	readonly file: string;

	/** What went wrong. */
	readonly message: string;
};

/** The measurement of one file or of a whole run. */
export type ScanResult = {
	/** Every file the scan could not measure. */
	readonly errors: Array<ScanFailure>;

	/** Every function the scan scored. */
	readonly functions: Array<ScoredFunction>;
};

/** Mutable accumulator for one reporting key while sites are folded into it. */
type KeyDraft = {
	cognitive: number;
	cyclomatic: number;
	line: number;
	name: string;
	named: boolean;
};

/**
 * Scores every function in a source file under both metrics.
 *
 * A named function is one reporting key. Anonymous functions below it fold
 * their cognitive cost into that key (they are part of what the declaration
 * asks a reader to hold) but keep their own cyclomatic number, of which the
 * key reports the worst — so neither metric can be hidden behind a callback.
 */
export class ComplexityScanner {
	readonly #parser = new SourceParser();
	readonly #collector = new FunctionSiteCollector();
	readonly #cyclomatic = new CyclomaticScorer();
	readonly #cognitive = new CognitiveScorer();

	/**
	 * Score one source file.
	 *
	 * @param file - The repository-relative path used in keys and findings.
	 * @param text - The complete source text.
	 * @returns The file's scored functions, or the parse failure as a value.
	 */
	scan(file: string, text: string): ScanResult {
		const parsed = this.#parser.parse(file, text);

		if (isErr(parsed)) {
			return { errors: [{ file, message: parsed.error.message }], functions: [] };
		}

		const lines = LineIndex.of(text);
		const drafts = new Map<string, KeyDraft>();

		for (const site of this.#collector.collect(parsed.value.program)) {
			this.#fold(drafts, site, lines.lineAt(site.start));
		}

		return { errors: [], functions: this.#render(file, drafts) };
	}

	#fold(drafts: Map<string, KeyDraft>, site: FunctionSite, line: number): void {
		const key = this.#keyFor(drafts, site, line);
		const draft = drafts.get(key);
		const cyclomatic = this.#cyclomatic.score(site.node);

		if (!draft) {
			drafts.set(key, {
				cognitive: site.named ? this.#cognitive.score(site.node) : 0,
				cyclomatic,
				line,
				name: key,
				named: site.named,
			});

			return;
		}

		draft.cyclomatic = Math.max(draft.cyclomatic, cyclomatic);
	}

	/**
	 * Resolve the key a site reports under, keeping two same-named declarations
	 * apart by their line rather than silently merging them.
	 */
	#keyFor(drafts: Map<string, KeyDraft>, site: FunctionSite, line: number): string {
		const existing = drafts.get(site.name);

		if (site.named && existing?.named && existing.line !== line) {
			return `${site.name}:${line}`;
		}

		return site.name;
	}

	#render(file: string, drafts: Map<string, KeyDraft>): Array<ScoredFunction> {
		const functions: Array<ScoredFunction> = [];

		for (const [name, draft] of drafts) {
			functions.push({
				cognitive: draft.cognitive,
				cyclomatic: draft.cyclomatic,
				file,
				key: `${file}#${name}`,
				line: draft.line,
				name,
			});
		}

		return functions.sort((left, right) => {
			return left.line - right.line || left.name.localeCompare(right.name);
		});
	}
}
