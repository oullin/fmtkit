import { z } from 'zod';

/** A recursively validated non-node object carried by an AST property. */
export type AstRecord = {
	readonly [key: string]: AstValue;
};

/** A recursively validated value carried by an AST node or metadata object. */
export type AstValue = AstRecord | AstValue[] | Node | RegExp | bigint | boolean | null | number | string | undefined;

type RoutedAstValue =
	| { readonly kind: 'array'; readonly value: unknown }
	| { readonly kind: 'node'; readonly value: unknown }
	| { readonly kind: 'record'; readonly value: unknown }
	| { readonly kind: 'regexp'; readonly value: unknown }
	| { readonly kind: 'scalar'; readonly value: unknown };

class AstValueRouter {
	static route(value: unknown): RoutedAstValue {
		if (Array.isArray(value)) {
			return { kind: 'array', value };
		}

		if (value instanceof RegExp) {
			return { kind: 'regexp', value };
		}

		if (value instanceof Object) {
			return 'type' in value ? { kind: 'node', value } : { kind: 'record', value };
		}

		return { kind: 'scalar', value };
	}
}

const AstValueSchema: z.ZodType<AstValue> = z.lazy(() => {
	return z.preprocess(
		AstValueRouter.route,
		z.discriminatedUnion('kind', [
			z.object({ kind: z.literal('node'), value: Node.schema }).transform((routed) => routed.value),
			z.object({ kind: z.literal('array'), value: z.array(AstValueSchema) }).transform((routed) => routed.value),
			z.object({ kind: z.literal('record'), value: z.record(z.string(), AstValueSchema) }).transform((routed) => routed.value),
			z.object({ kind: z.literal('regexp'), value: z.instanceof(RegExp) }).transform((routed) => routed.value),
			z
				.object({
					kind: z.literal('scalar'),
					value: z.union([z.bigint(), z.boolean(), z.null(), z.number(), z.string(), z.undefined()]),
				})
				.transform((routed) => routed.value),
		]),
	);
});

/** An immutable AST node produced by the recursive Oxc boundary schema. */
export class Node {
	readonly [key: string]: AstValue;

	/** The Oxc node discriminator. */
	readonly type: string;

	/** The inclusive source start when supplied by Oxc. */
	readonly start: number | undefined;

	/** The exclusive source end when supplied by Oxc. */
	readonly end: number | undefined;

	/** The source range fallback when supplied by Oxc. */
	readonly range: [number, number] | undefined;

	/** A schema-derived string-valued `name` property. */
	readonly name: string | undefined;

	/** A schema-derived string-valued `kind` property. */
	readonly kind: string | undefined;

	/** A schema-derived string-valued literal, when present. */
	readonly stringValue: string | undefined;

	/** The recursive schema used once at the parser boundary. */
	static readonly schema: z.ZodType<Node> = z.lazy(() => {
		return z
			.object({
				type: z.string(),
				start: z.number().optional(),
				end: z.number().optional(),
				range: z.tuple([z.number(), z.number()]).optional(),
				name: z.string().optional().catch(undefined),
				kind: z.string().optional().catch(undefined),
			})
			.catchall(AstValueSchema)
			.transform((value) => {
				return Node.#fromValidated(value);
			});
	});

	private constructor(value: Record<string, AstValue> & { type: string }) {
		Object.assign(this, value);

		this.type = value.type;
		this.start = Node.#numberProperty(value.start);
		this.end = Node.#numberProperty(value.end);
		this.range = Node.#rangeProperty(value.range);
		this.name = Node.#stringProperty(value.name);
		this.kind = Node.#stringProperty(value.kind);
		this.stringValue = Node.#stringProperty(value.value);

		Object.freeze(this);
	}

	static #fromValidated(value: Record<string, AstValue> & { type: string }): Node {
		return new Node(value);
	}

	static #numberProperty(value: AstValue): number | undefined {
		const parsed = z.number().safeParse(value);

		return parsed.success ? parsed.data : undefined;
	}

	static #rangeProperty(value: AstValue): [number, number] | undefined {
		const parsed = z.tuple([z.number(), z.number()]).safeParse(value);

		return parsed.success ? parsed.data : undefined;
	}

	static #stringProperty(value: AstValue): string | undefined {
		const parsed = z.string().safeParse(value);

		return parsed.success ? parsed.data : undefined;
	}
}

/** The complete parser payload validated before formatting passes see it. */
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
	 * Parse an Oxc program/comments payload once into its immutable DTO.
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
