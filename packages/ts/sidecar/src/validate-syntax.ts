import { pathToFileURL } from 'node:url';
import { z } from 'zod';
import type { OxcErrorDto } from '#sidecar/kernel/errors';
import { FormatPipeline } from '#sidecar/pipeline/format-pipeline';
import { NodeProcessRunner } from '#sidecar/io/process-runner';
import { NodeSourceFiles } from '#sidecar/io/source-files';
import { PipelineFactory } from '#sidecar/pipeline/pipeline-factory';
import { SourceFileEditor } from '#sidecar/pipeline/source-file-editor';

/** Immutable command-line options for standalone syntax validation. */
export class SyntaxCliDto {
	/** TypeScript and Vue files eligible for syntax validation. */
	readonly files: readonly string[];

	static readonly #argvSchema = z.array(z.string());

	static readonly #schema = z.object({
		files: z.array(z.string()),
	});

	private constructor(value: { files: string[] }) {
		this.files = Object.freeze(value.files);

		Object.setPrototypeOf(this, Object.prototype);
		Object.freeze(this);
	}

	/**
	 * Parse the standalone syntax-validation command line.
	 *
	 * @param input - Arguments after the executable and script path.
	 * @returns Immutable syntax-validation options.
	 */
	static parse(input: unknown): SyntaxCliDto {
		const argv = SyntaxCliDto.#argvSchema.parse(input);

		const files = argv.filter((file) => {
			return file.endsWith('.ts') || file.endsWith('.vue');
		});

		return new SyntaxCliDto(SyntaxCliDto.#schema.parse({ files }));
	}
}

/** Formats parser diagnostics for the standalone syntax-validation command. */
class SyntaxErrorReporter {
	/**
	 * Format one parser diagnostic for console output.
	 *
	 * @param file - The source path associated with the diagnostic.
	 * @param error - The parser diagnostic to render.
	 * @returns A source-framed message, plain message, or stable fallback.
	 */
	static format(file: string, error: OxcErrorDto): string {
		if (error.codeframe && error.codeframe.length > 0) {
			return `[validate-syntax] ${file}\n${error.codeframe.trimEnd()}`;
		}

		if (error.message && error.message.length > 0) {
			return `[validate-syntax] ${file}: ${error.message}`;
		}

		return `[validate-syntax] ${file}: syntax validation failed`;
	}
}

async function main(): Promise<void> {
	const cwd = process.cwd();
	const options = SyntaxCliDto.parse(process.argv.slice(2));
	const files = [...options.files];
	const factory = PipelineFactory.create();
	const sourceFiles = new NodeSourceFiles();

	const pipeline = new FormatPipeline({
		editor: new SourceFileEditor({ sourceFiles }),
		processRunner: new NodeProcessRunner(),
		validator: factory.syntaxValidator(sourceFiles),
	});

	const failures = await pipeline.validate(files);

	const diagnostics: string[] = [];

	for (const failure of failures) {
		if (failure.error._tag === 'SourceFileUnreadable') {
			if (failure.error.isNotFound()) {
				console.warn(`[validate-syntax] path not found, skipping: ${failure.file}`);

				continue;
			}

			console.error(failure.error);
			process.exitCode = 1;

			return;
		}

		for (const error of failure.error.errors) {
			diagnostics.push(SyntaxErrorReporter.format(failure.file, error));
		}
	}

	if (diagnostics.length > 0) {
		console.error(diagnostics.join('\n'));
		console.error(`[validate-syntax] ${diagnostics.length} syntax error(s) found after formatting.`);
		process.exitCode = 1;

		return;
	}

	console.log(`[validate-syntax] checked ${files.length} file(s) in ${cwd}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	main().catch((error: unknown) => {
		console.error(error);
		process.exitCode = 1;
	});
}
