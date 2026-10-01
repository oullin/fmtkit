/** A fenced code block embedded in a Markdown document. */
export type MarkdownFenceBlock = {
	/** The source text of the fenced block body. */
	readonly content: string;

	/** The first token of the opening fence info string, or an empty string. */
	readonly lang: string;

	/** The source offset where `content` starts in the Markdown source. */
	readonly start: number;
};

/** A source line paired with its byte offsets. */
type ScannedLine = {
	/** The offset immediately after the line terminator. */
	readonly end: number;

	/** The offset where the line begins. */
	readonly start: number;

	/** The line text without its trailing carriage return or newline. */
	readonly text: string;
};

type OpeningFence = {
	readonly fenceChar: string;
	readonly fenceLength: number;
	readonly info: string;
	readonly line: ScannedLine;
};

const OPEN_FENCE_REGEX = /^( {0,3})(`{3,}|~{3,})(.*)$/;
const JAVASCRIPT_LANGS = ['ts', 'tsx', 'js', 'jsx', 'typescript', 'javascript', 'mjs', 'cjs', 'mts', 'cts'];

/** Inspects fenced code blocks embedded in CommonMark documents. */
export class MarkdownFences {
	#scanLines(content: string): Array<ScannedLine> {
		const lines: Array<ScannedLine> = [];

		let position = 0;

		while (true) {
			const newline = content.indexOf('\n', position);

			if (newline === -1) {
				lines.push({ end: content.length, start: position, text: this.#stripCarriageReturn(content.slice(position)) });

				return lines;
			}

			lines.push({ end: newline + 1, start: position, text: this.#stripCarriageReturn(content.slice(position, newline)) });
			position = newline + 1;
		}
	}

	#stripCarriageReturn(text: string): string {
		return text.endsWith('\r') ? text.slice(0, -1) : text;
	}

	#infoLanguage(info: string): string {
		return info.trim().split(/\s+/)[0] ?? '';
	}

	#findClose(lines: Array<ScannedLine>, from: number, fenceChar: string, minLength: number): number {
		const pattern = new RegExp(`^ {0,3}${fenceChar}{${minLength},}[ \\t]*$`);

		for (let index = from; index < lines.length; index++) {
			const line = lines[index];

			if (line && pattern.test(line.text)) {
				return index;
			}
		}

		return -1;
	}

	#openingFence(line: ScannedLine | undefined): OpeningFence | null {
		if (!line) {
			return null;
		}

		const match = OPEN_FENCE_REGEX.exec(line.text);

		if (!match) {
			return null;
		}

		const fence = match[2] ?? '';
		const fenceChar = fence[0] ?? '`';
		const info = match[3] ?? '';

		if (fenceChar === '`' && info.includes('`')) {
			return null;
		}

		return { fenceChar, fenceLength: fence.length, info, line };
	}

	/**
	 * Extract every JavaScript-capable Markdown fence body and its source offset.
	 *
	 * The scanner recognises opening fences of three or more backticks or tildes,
	 * preceded by up to three spaces of indentation, whose body runs until a
	 * matching closing fence of the same character and at least the same length.
	 * Unterminated fences run to the end of the document and yield no block.
	 *
	 * @param content - The complete Markdown source text.
	 * @returns The embedded fence blocks in source order.
	 */
	extractBlocks(content: string): Array<MarkdownFenceBlock> {
		const blocks: Array<MarkdownFenceBlock> = [];
		const lines = this.#scanLines(content);

		let index = 0;

		while (index < lines.length) {
			const opening = this.#openingFence(lines[index]);

			if (!opening) {
				index++;

				continue;
			}

			const closeIndex = this.#findClose(lines, index + 1, opening.fenceChar, opening.fenceLength);

			if (closeIndex === -1) {
				break;
			}

			const bodyStart = opening.line.end;
			const bodyEnd = lines[closeIndex]?.start ?? content.length;

			blocks.push({
				content: content.slice(bodyStart, bodyEnd),
				lang: this.#infoLanguage(opening.info),
				start: bodyStart,
			});

			index = closeIndex + 1;
		}

		return blocks;
	}

	/**
	 * Report whether a fence info language denotes JavaScript or TypeScript.
	 *
	 * @param lang - The first token of the fence info string.
	 * @returns `true` for JavaScript and TypeScript language identifiers.
	 */
	isJavaScriptOrTypeScript(lang: string): boolean {
		return JAVASCRIPT_LANGS.includes(lang.toLowerCase());
	}

	/**
	 * Select the parser extension for a fenced block language.
	 *
	 * @param lang - The first token of the fence info string.
	 * @returns `tsx` for JSX-flavoured languages, otherwise `ts`.
	 */
	scriptExtension(lang: string): 'ts' | 'tsx' {
		const normalized = lang.toLowerCase();

		return normalized === 'tsx' || normalized === 'jsx' ? 'tsx' : 'ts';
	}
}
