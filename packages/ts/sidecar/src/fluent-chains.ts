import { pathToFileURL } from 'node:url';
import { AstReader } from '#sidecar/syntax/ast-reader';
import { DrizzleQueries } from '#sidecar/drizzle-queries';
import { EditApplier } from '#sidecar/syntax/edits';
import { EmbeddedBlocks } from '#sidecar/hosts/embedded-blocks';
import { ExpandedCalls } from '#sidecar/expanded-calls';
import { PassCliDto } from '#sidecar/pass-cli-dto';
import { isErr, ok } from '#sidecar/kernel/result';
import type { ParsedSourceDto } from '#sidecar/syntax/node-schema';
import type { Result } from '#sidecar/kernel/result';
import type { SourceFileError, SourceFiles } from '#sidecar/io/source-files';
import { SourceDocument } from '#sidecar/syntax/source-document';
import { SourceParser } from '#sidecar/syntax/source-parser';
import type { Edit } from '#sidecar/syntax/edits';
import type { Node } from '#sidecar/syntax/node-schema';

const cwd = process.cwd();

type ChainLink = {
	start: number;
	end: number;
	operator: '.' | '?.';
};

type FluentChain = {
	base: Node;
	links: ChainLink[];
};

/** Formats fluent chains and the structured calls composed with them. */
export class FluentChains {
	static readonly #ast = new AstReader();

	static readonly #editApplier = new EditApplier();

	static readonly #parser = new SourceParser();

	static #memberCallLink(document: SourceDocument, member: Node, object: Node, parsed: ParsedSourceDto): ChainLink | null {
		if (member.computed) {
			return null;
		}

		const property = FluentChains.#ast.childNode(member, 'property');

		if (!property || (property.type !== 'Identifier' && property.type !== 'PrivateIdentifier')) {
			return null;
		}

		const objectEnd = FluentChains.#ast.getEnd(object);
		const propertyStart = FluentChains.#ast.getStart(property);

		if (objectEnd < 0 || propertyStart < 0 || propertyStart <= objectEnd) {
			return null;
		}

		if (parsed.hasCommentBetween(objectEnd, propertyStart)) {
			return null;
		}

		const separator = document.slice(objectEnd, propertyStart);

		if (separator.includes('//') || separator.includes('/*')) {
			return null;
		}

		const operator = separator.replace(/[ \t\r\n]/g, '');

		if (operator !== '.' && operator !== '?.') {
			return null;
		}

		return {
			start: objectEnd,
			end: propertyStart,
			operator,
		};
	}

	static #collectFluentChain(document: SourceDocument, outer: Node, parsed: ParsedSourceDto): FluentChain | null {
		let call: Node = outer;

		const links: ChainLink[] = [];

		while (call.type === 'CallExpression') {
			const callee = FluentChains.#ast.unwrapChainExpression(FluentChains.#ast.childNode(call, 'callee'));

			if (callee?.type !== 'MemberExpression') {
				break;
			}

			const object = FluentChains.#ast.unwrapChainExpression(FluentChains.#ast.childNode(callee, 'object'));

			if (object?.type !== 'CallExpression') {
				break;
			}

			const link = FluentChains.#memberCallLink(document, callee, object, parsed);

			if (!link) {
				return null;
			}

			links.push(link);
			call = object;
		}

		if (links.length < 2) {
			return null;
		}

		return {
			base: call,
			links,
		};
	}
	/**
	 * Compute edits that split fluent-chain links across lines.
	 *
	 * @param content - The source text to inspect.
	 * @param virtualName - The filename used to parse the source.
	 * @returns Fluent-chain edits, or none for invalid source.
	 */
	static computeEdits(content: string, virtualName: string): Edit[] {
		const parsed = FluentChains.#parser.parse(virtualName, content);

		if (isErr(parsed)) {
			return [];
		}

		const document = SourceDocument.of(virtualName, content);
		const edits = new Map<string, Edit>();
		const indentStep = document.indentUnit();

		FluentChains.#ast.visit(parsed.value.program, (node) => {
			if (node.type !== 'CallExpression') {
				return;
			}

			const chain = FluentChains.#collectFluentChain(document, node, parsed.value);

			if (!chain) {
				return;
			}

			const baseStart = FluentChains.#ast.getStart(chain.base);

			if (baseStart < 0) {
				return;
			}

			const indent = `${document.lineIndent(baseStart)}${indentStep}`;

			for (const link of chain.links) {
				const replacement = `\n${indent}${link.operator}`;

				if (document.slice(link.start, link.end) === replacement) {
					continue;
				}

				edits.set(`${link.start}:${link.end}`, {
					start: link.start,
					end: link.end,
					replacement,
				});
			}
		});

		return [...edits.values()].sort((a, b) => {
			return a.start - b.start;
		});
	}

	/**
	 * Apply fluent-chain, Drizzle-query, and expanded-call formatting.
	 *
	 * @param content - The source text to format.
	 * @param virtualName - The filename used to parse the source.
	 * @returns The formatted source text.
	 */
	static format(content: string, virtualName: string): string {
		const edits = FluentChains.computeEdits(content, virtualName);

		const fluentFormatted = edits.length > 0 ? FluentChains.#editApplier.apply(content, edits) : content;
		const drizzleFormatted = DrizzleQueries.format(fluentFormatted, virtualName);

		return ExpandedCalls.format(drizzleFormatted, virtualName);
	}

	/**
	 * Format one TypeScript or host file through an injected filesystem port.
	 *
	 * @param file - The source file to format.
	 * @param mode - Whether to report changes or atomically write them.
	 * @param sourceFiles - The filesystem port used for reads and writes.
	 * @returns Whether the file changes, or the typed filesystem failure.
	 */
	static async formatFile(file: string, mode: 'check' | 'write', sourceFiles: SourceFiles): Promise<Result<boolean, SourceFileError>> {
		const read = await sourceFiles.readText(file);

		if (isErr(read)) {
			return read;
		}

		const original = read.value;

		const updated = EmbeddedBlocks.isHost(file)
			? EmbeddedBlocks.rewrite(file, original, (blockContent, virtualName) => {
					return FluentChains.format(blockContent, virtualName);
				})
			: FluentChains.format(original, file);

		if (updated === original) {
			return ok(false);
		}

		if (mode === 'write') {
			const written = await sourceFiles.writeTextAtomic(file, updated);

			if (isErr(written)) {
				return written;
			}
		}

		return ok(true);
	}

	/**
	 * Run the standalone fluent-chain formatter entrypoint.
	 *
	 * @returns Nothing after reporting outcomes and setting the process status.
	 */
	static async main(): Promise<void> {
		const options = PassCliDto.parse(process.argv.slice(2));
		const files = [...options.files];
		const { mode } = options;

		const { NodeProcessRunner } = await import('#sidecar/io/process-runner');

		const { NodeSourceFiles } = await import('#sidecar/io/source-files');

		const { FormatPipeline } = await import('#sidecar/format-pipeline');

		const pipeline = new FormatPipeline({ sourceFiles: new NodeSourceFiles(), processRunner: new NodeProcessRunner() });

		const outcomes = await pipeline.runPass('fluent-chains', files, mode, (file, passMode) => {
			return pipeline.formatFluentFile(file, passMode);
		});

		const changedCount = outcomes.filter((outcome) => {
			if (outcome.error?._tag === 'SourceFileUnreadable' && outcome.error.isNotFound()) {
				console.warn(`[fluent-chains] path not found, skipping: ${outcome.file}`);
			} else if (outcome.error) {
				throw outcome.error;
			} else if (outcome.changed) {
				console.log(`[fluent-chains] ${mode === 'check' ? 'would change' : 'updated'} ${outcome.file}`);
			}

			return outcome.changed;
		}).length;

		if (mode === 'check' && changedCount > 0) {
			console.error(`[fluent-chains] ${changedCount} file(s) need fluent-chain edits. Run "pnpm format" to fix.`);
			process.exit(1);
		}

		console.log(`[fluent-chains] processed ${files.length} file(s) in ${cwd}, ${changedCount} ${mode === 'check' ? 'would change' : 'changed'}`);
	}
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	FluentChains.main().catch((err: unknown) => {
		console.error(err);
		process.exit(1);
	});
}
