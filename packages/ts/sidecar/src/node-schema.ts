import { z } from 'zod';

/** A trusted non-node object carried by an Oxc AST property. */
export type AstRecord = {
	readonly [key: string]: AstValue;
};

/** A value carried by an eagerly validated node head or trusted descendant. */
export type AstValue = AstRecord | AstValue[] | Node | RegExp | bigint | boolean | null | number | string | undefined;

const NodeHeadSchema = z
	.object({
		type: z.string(),
		start: z.number().optional(),
		end: z.number().optional(),
		range: z.tuple([z.number(), z.number()]).optional(),
		name: z.string()
			.optional()
			.catch(undefined),
		kind: z.string()
			.optional()
			.catch(undefined),
	})
	.passthrough();

/**
 * An Oxc AST node admitted by the shallow parser-boundary schema.
 *
 * `schema` eagerly validates a root or comment node's discriminator and common
 * positional head, while retaining other properties. Program descendants,
 * including their discriminators, positions, and nested structure, are trusted
 * Oxc output after that envelope succeeds and are recognised structurally.
 * `Ast` lazily Zod-validates the descendant `name`, `kind`, and string `value`
 * fields that passes consume, avoiding a recursive walk and reconstruction.
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
		return Object.freeze(value) as Node;
	});

	private constructor() {}

	/** Recognise trusted descendant nodes without recursively materialising them. */
	static [Symbol.hasInstance](value: unknown): boolean {
		return value instanceof Object && 'type' in value;
	}
}

/**
 * The parser payload admitted before formatting passes can consume it.
 *
 * The DTO schema eagerly validates the payload envelope, the program node
 * head, the comments array, and every comment node head. Program descendant
 * structure and positions remain trusted Oxc data; narrow `Ast` readers lazily
 * validate consumed `name`, `kind`, and string `value` fields. No pass receives
 * the raw parser payload.
 */
export class ParsedSourceDto {
	/** The parsed program root. */
	readonly program: Node;

	/** Parsed comments represented as traversable nodes. */
	readonly comments: readonly Node[];

	static readonly #schema = z.object({
		program: Node.schema,
		comments: z.array(Node.schema),
	});

	private constructor(program: Node, comments: Node[]) {
		this.program = program;
		this.comments = Object.freeze(comments);

		Object.freeze(this);
	}

	/**
	 * Validate an Oxc program/comments envelope once into its immutable DTO.
	 *
	 * @param value - The untrusted parser payload.
	 * @returns The validated DTO, or the Zod validation failure.
	 */
	static from(value: unknown): z.ZodSafeParseResult<ParsedSourceDto> {
		const parsed = ParsedSourceDto.#schema.safeParse(value);

		if (!parsed.success) {
			return parsed;
		}

		return {
			success: true,
			data: new ParsedSourceDto(parsed.data.program, parsed.data.comments),
		};
	}
}
