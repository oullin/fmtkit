import type { EmbeddedBlockSplitter } from '#sidecar/hosts/embedded-block-splitter';

/** Classifies paths accepted by sidecar formatting passes. */
export class FileTargetPolicy {
	readonly #embeddedBlocks: EmbeddedBlockSplitter;

	/**
	 * @param dependencies - The collaborators the policy classifies through.
	 * @param dependencies.embeddedBlocks - Recognises host documents that embed JavaScript.
	 */
	constructor(dependencies: { embeddedBlocks: EmbeddedBlockSplitter }) {
		this.#embeddedBlocks = dependencies.embeddedBlocks;
	}

	/**
	 * Report whether a virtual filename denotes a TypeScript declaration file.
	 *
	 * @param virtualName - The filename to classify.
	 * @returns `true` when the filename ends in `.d.ts`.
	 */
	isDeclarationFile(virtualName: string): boolean {
		return virtualName.endsWith('.d.ts');
	}

	/**
	 * Report whether a path denotes a supported non-declaration source file.
	 *
	 * @param path - The source path to classify.
	 * @returns `true` for host documents and non-declaration TypeScript files.
	 */
	isTargetFile(path: string): boolean {
		return (path.endsWith('.ts') && !path.endsWith('.d.ts')) || this.#embeddedBlocks.isHost(path);
	}

	/**
	 * Report whether a path is eligible for final syntax validation.
	 *
	 * @param path - The source path to classify.
	 * @returns `true` for host documents and every TypeScript file.
	 */
	isSyntaxTarget(path: string): boolean {
		return path.endsWith('.ts') || this.#embeddedBlocks.isHost(path);
	}
}
