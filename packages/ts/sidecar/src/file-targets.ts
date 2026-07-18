/** Report whether a virtual filename denotes a TypeScript declaration file. */
export function isDeclarationFile(virtualName: string): boolean {
	return virtualName.endsWith('.d.ts');
}

/** Report whether a path denotes a supported non-declaration source file. */
export function isTargetFile(path: string): boolean {
	return (path.endsWith('.ts') && !path.endsWith('.d.ts')) || path.endsWith('.vue');
}
