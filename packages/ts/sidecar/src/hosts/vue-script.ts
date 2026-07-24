/** A script block embedded in a Vue single-file component. */
export type VueScriptBlock = {
	/** The complete opening `<script>` tag. */
	readonly openTag: string;

	/** The source text between the opening and closing tags. */
	readonly content: string;

	/** The source offset where `content` starts in the Vue source. */
	readonly start: number;
};

const VUE_SCRIPT_REGEX = /(<script\b[^>]*>)([\s\S]*?)(<\/script>)/g;

/** Inspects script blocks embedded in Vue single-file components. */
export class VueScript {
	/**
	 * Extract every Vue script block and its source offset.
	 *
	 * @param content - The complete Vue source text.
	 * @returns The embedded script blocks in source order.
	 */
	extractBlocks(content: string): VueScriptBlock[] {
		const blocks: VueScriptBlock[] = [];

		VUE_SCRIPT_REGEX.lastIndex = 0;

		let match: RegExpExecArray | null;

		while ((match = VUE_SCRIPT_REGEX.exec(content)) !== null) {
			const openTag = match[1] ?? '';

			blocks.push({
				openTag,
				content: match[2] ?? '',
				start: match.index + openTag.length,
			});
		}

		return blocks;
	}

	/**
	 * Read an opening script tag attribute case-insensitively.
	 *
	 * @param openTag - The complete opening `<script>` tag.
	 * @param name - The attribute name to read.
	 * @returns The lower-cased value, or `null` when the attribute has no value.
	 */
	attribute(openTag: string, name: string): string | null {
		const pattern = new RegExp(`\\b${name}\\s*=\\s*(?:"([^"]*)"|'([^']*)'|([^\\s>]+))`, 'i');
		const match = openTag.match(pattern);
		const value = match ? (match[1] ?? match[2] ?? match[3]) : undefined;

		return value === undefined ? null : value.toLowerCase();
	}

	/**
	 * Report whether a Vue script tag carries JavaScript or TypeScript.
	 *
	 * @param openTag - The complete opening `<script>` tag.
	 * @returns `true` for JavaScript, TypeScript, and module script tags.
	 */
	isJavaScriptOrTypeScript(openTag: string): boolean {
		const lang = this.attribute(openTag, 'lang');

		if (lang) {
			return ['ts', 'tsx', 'js', 'jsx', 'typescript', 'javascript'].includes(lang);
		}

		const type = this.attribute(openTag, 'type');

		if (type) {
			return type === 'module' || type.includes('javascript') || type.includes('ecmascript');
		}

		return true;
	}
}
