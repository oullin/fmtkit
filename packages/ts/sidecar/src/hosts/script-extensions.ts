/**
 * The TypeScript-family extension taxonomy, shared so the formatting passes and
 * the standalone syntax CLI cannot drift apart on which files they accept.
 *
 * The extension selects the parser dialect, so `.tsx` is what turns JSX on: a
 * `.tsx` file classified as `.ts` fails to parse the moment it holds an element.
 */
export class ScriptExtensions {
	/** TypeScript-family extensions oxc parses directly. */
	static readonly #script = ['.ts', '.tsx', '.mts', '.cts'];

	/** Declaration forms. There is no `.d.tsx`: JSX cannot appear in one. */
	static readonly #declaration = ['.d.ts', '.d.mts', '.d.cts'];

	/**
	 * Report whether a path denotes a TypeScript-family script.
	 *
	 * @param path - The path to classify.
	 * @returns `true` for `.ts`, `.tsx`, `.mts`, and `.cts`, declarations included.
	 */
	static isScript(path: string): boolean {
		return ScriptExtensions.#script.some((suffix) => path.endsWith(suffix));
	}

	/**
	 * Report whether a path denotes a TypeScript declaration file.
	 *
	 * @param path - The path to classify.
	 * @returns `true` for `.d.ts`, `.d.mts`, and `.d.cts`.
	 */
	static isDeclaration(path: string): boolean {
		return ScriptExtensions.#declaration.some((suffix) => path.endsWith(suffix));
	}
}
