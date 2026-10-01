import { z } from 'zod';

/** A trusted non-node object carried by an Oxc AST property. */
export type AstRecord = {
	readonly [key: string]: AstValue;
};

/** A value carried by an eagerly validated node head or trusted descendant. */
export type AstValue = AstRecord | Array<AstValue> | Node | RegExp | bigint | boolean | null | number | string | undefined;

const NodeHeadSchema = z
	.object({
		end: z.number().optional(),
		// oxlint-disable-next-line unicorn/prefer-top-level-await -- Zod catch handles parse failures, not Promise rejection.
		kind: z.string()
			.optional()
			.catch(undefined),
		// oxlint-disable-next-line unicorn/prefer-top-level-await -- Zod catch handles parse failures, not Promise rejection.
		name: z.string()
			.optional()
			.catch(undefined),
		range: z.tuple([z.number(), z.number()]).optional(),
		start: z.number().optional(),
		type: z.string(),
	})
	.passthrough();

/**
 * An Oxc AST node admitted by the shallow parser-boundary schema.
 *
 * `schema` eagerly validates a root or comment node's discriminator and common
 * positional head, while retaining other properties. Program descendants,
 * including their discriminators, positions, and nested structure, are trusted
 * Oxc output after that envelope succeeds and are recognised structurally.
 * `AstReader` lazily Zod-validates the descendant `name`, `kind`, and string
 * `value` fields that passes consume, avoiding a recursive walk and reconstruction.
 */
export class Node {
	readonly [key: string]: AstValue;

	/** The Oxc node discriminator. */
	declare readonly type: string;

	/** The inclusive source start when supplied by Oxc. */
	declare readonly start: number | undefined;

	/** The exclusive source end when supplied by Oxc. */
	declare readonly end: number | undefined;

	/** The source range fallback when supplied by Oxc. */
	declare readonly range: [number, number] | undefined;

	/** A string-valued `name` property when supplied by Oxc. */
	declare readonly name: string | undefined;

	/** A string-valued `kind` property when supplied by Oxc. */
	declare readonly kind: string | undefined;

	/** The shallow schema used once at the parser boundary. */
	static readonly schema: z.ZodType<Node> = NodeHeadSchema.transform((value) => {
		// SAFETY: NodeHeadSchema has just validated the discriminator and
		// positional head, so the frozen passthrough object satisfies the
		// shallow Node contract this schema admits.
		return Object.freeze(value) as Node;
	});

	private constructor() {}

	/**
	 * Recognise trusted descendant nodes without recursively materialising them.
	 *
	 * @param value - The possible AST node.
	 * @returns `true` when the value is an object carrying a node discriminator.
	 */
	static is(value: AstValue): value is Node {
		// oxlint-disable-next-line anti-slop/no-runtime-typeof -- Oxc descendants are admitted at the parser boundary; traversal must distinguish nested node records.
		return value !== null && typeof value === 'object' && 'type' in value;
	}
}

/**
 * The parser payload admitted before formatting passes can consume it.
 *
 * The DTO schema eagerly validates the payload envelope, the program node
 * head, the comments array, and every comment node head. Program descendant
 * structure and positions remain trusted Oxc data; narrow `AstReader` readers
 * lazily validate consumed `name`, `kind`, and string `value` fields. No pass receives
 * the raw parser payload.
 */
export class ParsedSourceDto {
	/** The parsed program root. */
	readonly program: Node;

	/** Parsed comments represented as traversable nodes. */
	readonly comments: ReadonlyArray<Node>;

	static readonly #schema = z.object({
		comments: z.array(Node.schema),
		program: Node.schema,
	});

	private constructor(program: Node, comments: Array<Node>) {
		this.program = program;
		this.comments = Object.freeze(comments);

		Object.freeze(this);
	}

	/**
	 * Validate an Oxc program/comments envelope once into its immutable DTO.
	 *
	 * The failure branch carries the raw Zod error; its payload type stays
	 * unparameterised because the DTO's own methods are not part of the schema
	 * output and no caller reads the typed error.
	 *
	 * @param value - The untrusted parser payload.
	 * @returns The validated DTO, or the Zod validation failure.
	 */
	// oxlint-disable-next-line anti-slop/no-unknown-parameters -- this DTO is the Zod boundary parser the rule routes callers toward; it must admit arbitrary payloads in order to reject them.
	static from(value: unknown): { data: ParsedSourceDto; success: true } | { error: z.ZodError; success: false } {
		const parsed = ParsedSourceDto.#schema.safeParse(value);

		if (!parsed.success) {
			return { error: parsed.error, success: false };
		}

		return {
			data: new ParsedSourceDto(parsed.data.program, parsed.data.comments),
			success: true,
		};
	}

	/**
	 * Report whether a complete comment lies between two offsets.
	 *
	 * @param from - The inclusive lower offset.
	 * @param to - The inclusive upper offset.
	 * @returns `true` when a comment is contained by the range.
	 */
	hasCommentBetween(from: number, to: number): boolean {
		return this.comments.some((comment) => {
			const start = comment.start ?? comment.range?.[0] ?? -1;
			const end = comment.end ?? comment.range?.[1] ?? -1;

			return start >= from && end <= to;
		});
	}
}
