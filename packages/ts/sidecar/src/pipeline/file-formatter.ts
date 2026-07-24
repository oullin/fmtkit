import type { EmbeddedBlockSplitter } from '#sidecar/hosts/embedded-block-splitter';
import type { PassPipeline } from '#sidecar/pipeline/pass-pipeline';
import { SourceDocument } from '#sidecar/syntax/source-document';

/** Applies a pass pipeline to a file, rewriting embedded blocks of host documents. */
export class FileFormatter {
	/** The reporting label carried from the underlying pipeline. */
	readonly label: string;

	readonly #splitter: EmbeddedBlockSplitter;
	readonly #pipeline: PassPipeline;

	/**
	 * @param dependencies - The host splitter and pipeline composed by the formatter.
	 * @param dependencies.splitter - Extracts and rewrites host embedded blocks.
	 * @param dependencies.pipeline - The pass pipeline applied to each source unit.
	 */
	constructor(dependencies: { splitter: EmbeddedBlockSplitter; pipeline: PassPipeline }) {
		this.#splitter = dependencies.splitter;
		this.#pipeline = dependencies.pipeline;
		this.label = dependencies.pipeline.name;
	}

	/**
	 * Format a file's text, dispatching host documents through embedded blocks.
	 *
	 * @param path - The source path, used to select host handling and syntax.
	 * @param content - The complete source text.
	 * @returns The formatted source text.
	 */
	format(path: string, content: string): string {
		if (this.#splitter.isHost(path)) {
			return this.#splitter.rewrite(path, content, (blockContent, virtualName) => {
				return this.#run(virtualName, blockContent);
			});
		}

		return this.#run(path, content);
	}

	#run(virtualName: string, content: string): string {
		return this.#pipeline.apply(SourceDocument.of(virtualName, content)).text;
	}
}
