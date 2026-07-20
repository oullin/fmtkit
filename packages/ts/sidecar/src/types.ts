/** The AST node type admitted by the parser boundary. */
export type { Node } from '#sidecar/node-schema';

/** A source replacement expressed against the original text offsets. */
export type Edit = {
	/** The inclusive replacement start. */
	start: number;

	/** The exclusive replacement end. */
	end: number;

	/** The text inserted in place of the selected range. */
	replacement: string;
};
