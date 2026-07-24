import { pathToFileURL } from 'node:url';
import { Ast } from '#sidecar/syntax/ast';
import { DrizzleQueries } from '#sidecar/drizzle-queries';
import { Edits } from '#sidecar/syntax/edits';
import { EmbeddedBlocks } from '#sidecar/hosts/embedded-blocks';
import { ExpandedCalls } from '#sidecar/expanded-calls';
import { PassCliDto } from '#sidecar/pass-cli-dto';
import { isErr, ok } from '#sidecar/kernel/result';
import type { Result } from '#sidecar/kernel/result';
import type { SourceFileError, SourceFiles } from '#sidecar/io/source-files';
import { SourceText } from '#sidecar/syntax/source-text';
import { Sources } from '#sidecar/syntax/sources';
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
	static #memberCallLink(source: string, member: Node, object: Node, comments: readonly Node[]): ChainLink | null {
		if (member.computed) {
			return null;
		}

		const property = Ast.childNode(member, 'property');

		if (!property || (property.type !== 'Identifier' && property.type !== 'PrivateIdentifier')) {
			return null;
		}

		const objectEnd = Ast.getEnd(object);
		const propertyStart = Ast.getStart(property);

		if (objectEnd < 0 || propertyStart < 0 || propertyStart <= objectEnd) {
			return null;
		}

		if (SourceText.hasCommentBetween(comments, objectEnd, propertyStart)) {
			return null;
		}

		const separator = source.slice(objectEnd, propertyStart);

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

	static #collectFluentChain(source: string, outer: Node, comments: readonly Node[]): FluentChain | null {
		let call: Node = outer;

		const links: ChainLink[] = [];

		while (call.type === 'CallExpression') {
			const callee = SourceText.unwrapChainExpression(Ast.childNode(call, 'callee'));

			if (callee?.type !== 'MemberExpression') {
				break;
			}

			const object = SourceText.unwrapChainExpression(Ast.childNode(callee, 'object'));

			if (object?.type !== 'CallExpression') {
				break;
			}

			const link = FluentChains.#memberCallLink(source, callee, object, comments);

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
		const parsed = Sources.parse(virtualName, content);

		if (isErr(parsed)) {
			return [];
		}

		const comments = parsed.value.comments;
		const edits = new Map<string, Edit>();
		const indentStep = SourceText.detectIndentUnit(content);

		Ast.visit(parsed.value.program, (node) => {
			if (node.type !== 'CallExpression') {
				return;
			}

			const chain = FluentChains.#collectFluentChain(content, node, comments);

			if (!chain) {
				return;
			}

			const baseStart = Ast.getStart(chain.base);

			if (baseStart < 0) {
				return;
			}

			const indent = `${SourceText.lineIndent(content, baseStart)}${indentStep}`;

			for (const link of chain.links) {
				const replacement = `\n${indent}${link.operator}`;

				if (content.slice(link.start, link.end) === replacement) {
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

		const fluentFormatted = edits.length > 0 ? Edits.apply(content, edits) : content;
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
