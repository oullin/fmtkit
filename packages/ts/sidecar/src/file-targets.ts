/** Classifies paths accepted by sidecar formatting passes. */
export class FileTargets {
	/** Report whether a virtual filename denotes a TypeScript declaration file. */
	static isDeclarationFile(virtualName: string): boolean {
		return virtualName.endsWith('.d.ts');
	}

	/** Report whether a path denotes a supported non-declaration source file. */
	static isTargetFile(path: string): boolean {
		return (path.endsWith('.ts') && !path.endsWith('.d.ts')) || path.endsWith('.vue');
	}
}
