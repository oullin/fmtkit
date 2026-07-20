/** Classifies paths accepted by sidecar formatting passes. */
export class FileTargets {
	/**
	 * Report whether a virtual filename denotes a TypeScript declaration file.
	 *
	 * @param virtualName - The filename to classify.
	 * @returns `true` when the filename ends in `.d.ts`.
	 */
	static isDeclarationFile(virtualName: string): boolean {
		return virtualName.endsWith('.d.ts');
	}

	/**
	 * Report whether a path denotes a supported non-declaration source file.
	 *
	 * @param path - The source path to classify.
	 * @returns `true` for Vue files and non-declaration TypeScript files.
	 */
	static isTargetFile(path: string): boolean {
		return (path.endsWith('.ts') && !path.endsWith('.d.ts')) || path.endsWith('.vue');
	}
}
