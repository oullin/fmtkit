import { ApiExportMissing, WorkerImportUnrecognised } from '#oxfmt-inprocess/errors';
import { err, ok } from '#oxfmt-inprocess/result';
import type { Result } from '#oxfmt-inprocess/result';

/** How oxfmt's worker entry imports the functions the shim delegates to. */
const WORKER_IMPORT = /import\s*\{([^}]*)\}\s*from\s*["']([^"']+)["']/;

/**
 * One imported binding, in either form a bundler may emit: `x as localName`
 * (rolldown's minified re-export) or a bare `localName`.
 */
const BINDING = /^([A-Za-z_$][\w$]*)(?:\s+as\s+([A-Za-z_$][\w$]*))?$/;

/**
 * Where oxfmt's embedded-formatting functions live, and what each is exported
 * as.
 *
 * oxfmt's worker entry (`cli-worker.js`) is a two-line module that re-exports
 * four functions from a content-hashed API module, e.g.
 *
 * ```js
 * import { i as sortTailwindClasses, n as formatEmbeddedDoc, r as formatFile, t as formatEmbeddedCode } from './apis-CvFX8LhR.js';
 * ```
 *
 * Both the hash and the single-letter aliases are bundler output and change
 * between oxfmt releases, so they are read from the worker entry rather than
 * hard-coded. Parsing succeeds only when all four functions are present, which
 * is what lets the getters be total.
 */
export class ApiBindings {
	readonly #moduleSpecifier: string;
	readonly #formatFile: string;
	readonly #formatEmbeddedCode: string;
	readonly #formatEmbeddedDoc: string;
	readonly #sortTailwindClasses: string;

	private constructor(moduleSpecifier: string, formatFile: string, formatEmbeddedCode: string, formatEmbeddedDoc: string, sortTailwindClasses: string) {
		this.#moduleSpecifier = moduleSpecifier;
		this.#formatFile = formatFile;
		this.#formatEmbeddedCode = formatEmbeddedCode;
		this.#formatEmbeddedDoc = formatEmbeddedDoc;
		this.#sortTailwindClasses = sortTailwindClasses;
	}

	/**
	 * Read the API module and its export aliases out of oxfmt's worker entry.
	 *
	 * @param workerSource - The contents of oxfmt's `cli-worker.js`.
	 * @param workerPath - Where that file came from, for error reporting.
	 * @returns The parsed bindings, or the reason the worker entry was not recognised.
	 */
	static parse(workerSource: string, workerPath: string): Result<ApiBindings, ApiExportMissing | WorkerImportUnrecognised> {
		const workerImport = WORKER_IMPORT.exec(workerSource);

		if (workerImport === null) {
			return err(new WorkerImportUnrecognised(workerPath, 'cannot find the API import'));
		}

		const [, bindingList, moduleSpecifier] = workerImport;

		if (bindingList === undefined || moduleSpecifier === undefined) {
			return err(new WorkerImportUnrecognised(workerPath, 'the API import carries no bindings'));
		}

		const exportsByRole = new Map<string, string>();

		for (const binding of bindingList.split(',')) {
			const trimmed = binding.trim();

			// A trailing comma in the import splits into an empty last element.
			if (trimmed === '') {
				continue;
			}

			const parsed = BINDING.exec(trimmed);
			const exported = parsed?.[1];

			if (parsed === null || exported === undefined) {
				return err(new WorkerImportUnrecognised(workerPath, `unexpected binding "${trimmed}"`));
			}

			// A bare binding is imported under its own name, so it is its own role.
			exportsByRole.set(parsed[2] ?? exported, exported);
		}

		const formatFile = exportsByRole.get('formatFile');
		const formatEmbeddedCode = exportsByRole.get('formatEmbeddedCode');
		const formatEmbeddedDoc = exportsByRole.get('formatEmbeddedDoc');
		const sortTailwindClasses = exportsByRole.get('sortTailwindClasses');

		if (formatFile === undefined) {
			return err(new ApiExportMissing('formatFile', workerPath));
		}

		if (formatEmbeddedCode === undefined) {
			return err(new ApiExportMissing('formatEmbeddedCode', workerPath));
		}

		if (formatEmbeddedDoc === undefined) {
			return err(new ApiExportMissing('formatEmbeddedDoc', workerPath));
		}

		if (sortTailwindClasses === undefined) {
			return err(new ApiExportMissing('sortTailwindClasses', workerPath));
		}

		return ok(new ApiBindings(moduleSpecifier, formatFile, formatEmbeddedCode, formatEmbeddedDoc, sortTailwindClasses));
	}

	/** The module specifier the API functions are imported from. */
	get moduleSpecifier(): string {
		return this.#moduleSpecifier;
	}

	/** The name `formatFile` is exported as. */
	get formatFile(): string {
		return this.#formatFile;
	}

	/** The name `formatEmbeddedCode` is exported as. */
	get formatEmbeddedCode(): string {
		return this.#formatEmbeddedCode;
	}

	/** The name `formatEmbeddedDoc` is exported as. */
	get formatEmbeddedDoc(): string {
		return this.#formatEmbeddedDoc;
	}

	/** The name `sortTailwindClasses` is exported as. */
	get sortTailwindClasses(): string {
		return this.#sortTailwindClasses;
	}
}
