import type { AstReader } from '#sidecar/syntax/ast-reader';
import type { DrizzleArgumentWriter } from '#sidecar/passes/drizzle/drizzle-argument-writer';
import type { DrizzleCallClassifier } from '#sidecar/passes/drizzle/drizzle-call-classifier';
import type { DrizzleImportScanner } from '#sidecar/passes/drizzle/drizzle-import-scanner';
import type { Edit, EditApplier } from '#sidecar/syntax/edits';
import type { FileTargetPolicy } from '#sidecar/hosts/file-target-policy';
import { isErr } from '#sidecar/kernel/result';
import type { FormattingPass } from '#sidecar/passes/pass';
import type { SourceDocument } from '#sidecar/syntax/source-document';
import type { SourceParser } from '#sidecar/syntax/source-parser';

/**
 * Formats recognised Drizzle query structures without touching unrelated calls.
 *
 * The pass is pure orchestration: it parses the document, scans its Drizzle
 * imports, then walks every call the classifier approves and asks the writer for
 * the edit that expands it. Detection, vocabulary, and emission live in the
 * injected collaborators — no static state and no shared reader remain here.
 */
export class DrizzleQueryPass implements FormattingPass {
	/** The pass identity used for reporting. */
	readonly name = 'drizzle-queries';

	readonly #parser: SourceParser;
	readonly #ast: AstReader;
	readonly #edits: EditApplier;
	readonly #scanner: DrizzleImportScanner;
	readonly #classifier: DrizzleCallClassifier;
	readonly #writer: DrizzleArgumentWriter;
	readonly #targets: FileTargetPolicy;

	/**
	 * @param dependencies - The services and collaborators consumed by the pass.
	 * @param dependencies.parser - Parses source into a trustworthy tree.
	 * @param dependencies.ast - Traverses and reads validated node fields.
	 * @param dependencies.edits - Reduces candidate edits to a non-overlapping set.
	 * @param dependencies.scanner - Collects the Drizzle imports in scope.
	 * @param dependencies.classifier - Decides which calls may be formatted.
	 * @param dependencies.writer - Emits the edit that expands an approved call.
	 * @param dependencies.targets - Classifies declaration files the pass skips.
	 */
	constructor(dependencies: { parser: SourceParser; ast: AstReader; edits: EditApplier; scanner: DrizzleImportScanner; classifier: DrizzleCallClassifier; writer: DrizzleArgumentWriter; targets: FileTargetPolicy }) {
		this.#parser = dependencies.parser;
		this.#ast = dependencies.ast;
		this.#edits = dependencies.edits;
		this.#scanner = dependencies.scanner;
		this.#classifier = dependencies.classifier;
		this.#writer = dependencies.writer;
		this.#targets = dependencies.targets;
	}

	/**
	 * Compute edits for recognised Drizzle query structures.
	 *
	 * @param document - The document to inspect.
	 * @returns Non-overlapping query-formatting edits, or none for invalid source.
	 */
	computeEdits(document: SourceDocument): Edit[] {
		if (this.#targets.isDeclarationFile(document.virtualName)) {
			return [];
		}

		const parsed = this.#parser.parse(document.virtualName, document.text);

		if (isErr(parsed)) {
			return [];
		}

		const imports = this.#scanner.scan(parsed.value.program);

		if (imports.isEmpty) {
			return [];
		}

		const edits: Edit[] = [];
		const indentUnit = document.indentUnit();

		this.#ast.visit(parsed.value.program, (node) => {
			if (node.type !== 'CallExpression') {
				return;
			}

			if (this.#classifier.isDrizzleMethodCall(node, imports) || this.#classifier.isRelationalQueryCall(node, imports) || this.#classifier.isSetOperationCall(node, imports)) {
				const args = this.#ast.childNodes(node, 'arguments');

				if (this.#classifier.isSetOperationCall(node, imports) && args.length > 0 && args.length < 2) {
					return;
				}

				if (!this.#classifier.isSetOperationCall(node, imports) && !this.#classifier.shouldFormatMethodArguments(node, imports)) {
					return;
				}

				const edit = this.#writer.formatCall(document, node, imports, parsed.value, indentUnit);

				if (edit) {
					edits.push(edit);
				}
			}
		});

		return this.#edits.nonOverlapping(edits);
	}
}
