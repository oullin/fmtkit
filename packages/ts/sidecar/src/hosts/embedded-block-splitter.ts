import { MarkdownFences } from '#sidecar/hosts/markdown-fences';
import { VueScript } from '#sidecar/hosts/vue-script';

/** A JavaScript-capable block embedded in a host document. */
export type EmbeddedBlock = {
	/** The source text of the embedded block. */
	readonly content: string;

	/** The source offset where `content` starts in the host source. */
	readonly start: number;

	/** The parser extension used to lex the block. */
	readonly extension: 'ts' | 'tsx';
};

/** Rewrites one embedded block, given its content and virtual filename. */
export type EmbeddedTransform = (blockContent: string, virtualName: string) => string;

/** Extracts and rewrites embedded JavaScript blocks across every host format. */
export class EmbeddedBlockSplitter {
	readonly #vueScript: VueScript;
	readonly #markdownFences: MarkdownFences;

	/**
	 * @param scanners - The per-format scanners the splitter delegates extraction to.
	 * @param scanners.vueScript - Reads script blocks from Vue and HTML host markup.
	 * @param scanners.markdownFences - Reads fenced code blocks from Markdown hosts.
	 */
	constructor(scanners: { vueScript: VueScript; markdownFences: MarkdownFences }) {
		this.#vueScript = scanners.vueScript;
		this.#markdownFences = scanners.markdownFences;
	}

	/**
	 * Report whether a path denotes a document that embeds JavaScript blocks.
	 *
	 * @param path - The source path to classify.
	 * @returns `true` for Vue, HTML, and Markdown host documents.
	 */
	isHost(path: string): boolean {
		return this.#isMarkup(path) || this.#isMarkdown(path);
	}

	/**
	 * Report whether a host document must fail a run on invalid embedded syntax.
	 *
	 * @param path - The source path to classify.
	 * @returns `true` for Vue and HTML; `false` for best-effort Markdown fences.
	 */
	hardValidated(path: string): boolean {
		return this.#isMarkup(path);
	}

	/**
	 * Extract every JavaScript-capable embedded block from a host document.
	 *
	 * @param path - The host source path, used to select the extraction strategy.
	 * @param content - The complete host source text.
	 * @returns The embedded blocks in source order, with parser extensions.
	 */
	extract(path: string, content: string): EmbeddedBlock[] {
		if (this.#isMarkdown(path)) {
			return this.#markdownFences.extractBlocks(content)
				.filter((block) => {
					return this.#markdownFences.isJavaScriptOrTypeScript(block.lang);
				})
				.map((block) => {
					return { content: block.content, start: block.start, extension: this.#markdownFences.scriptExtension(block.lang) };
				});
		}

		if (!this.#isMarkup(path)) {
			return [];
		}

		return this.#vueScript.extractBlocks(content)
			.filter((block) => {
				return this.#vueScript.isJavaScriptOrTypeScript(block.openTag);
			})
			.map((block) => {
				return { content: block.content, start: block.start, extension: this.#markupExtension(block.openTag) };
			});
	}

	/**
	 * Apply a transform to each embedded block and splice the results back.
	 *
	 * Blocks are rewritten in reverse offset order so earlier offsets stay valid
	 * while later blocks are replaced.
	 *
	 * @param path - The host source path.
	 * @param content - The complete host source text.
	 * @param transform - The rewrite applied to each block's content.
	 * @returns The host source with every changed block spliced back in place.
	 */
	rewrite(path: string, content: string, transform: EmbeddedTransform): string {
		let updated = content;

		const blocks = this.extract(path, content);

		for (const block of [...blocks].reverse()) {
			const rewritten = transform(block.content, `${path}.script.${block.extension}`);

			if (rewritten === block.content) {
				continue;
			}

			updated = updated.slice(0, block.start) + rewritten + updated.slice(block.start + block.content.length);
		}

		return updated;
	}

	#isMarkup(path: string): boolean {
		return path.endsWith('.vue') || path.endsWith('.html') || path.endsWith('.htm');
	}

	#isMarkdown(path: string): boolean {
		return path.endsWith('.md') || path.endsWith('.markdown');
	}

	#markupExtension(openTag: string): 'ts' | 'tsx' {
		const lang = this.#vueScript.attribute(openTag, 'lang') ?? '';

		return lang === 'tsx' || lang === 'jsx' ? 'tsx' : 'ts';
	}
}
