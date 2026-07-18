import { pathToFileURL } from 'node:url';
import type { OxcError } from 'oxc-parser';
import { FormatPipeline } from '#sidecar/format-pipeline';
import { NodeProcessRunner } from '#sidecar/process-runner';
import { NodeSourceFiles } from '#sidecar/source-files';

function formatError(file: string, error: OxcError): string {
	if (typeof error.codeframe === 'string' && error.codeframe.length > 0) {
		return `[validate-syntax] ${file}\n${error.codeframe.trimEnd()}`;
	}

	if (typeof error.message === 'string' && error.message.length > 0) {
		return `[validate-syntax] ${file}: ${error.message}`;
	}

	return `[validate-syntax] ${file}: syntax validation failed`;
}

async function main(): Promise<void> {
	const cwd = process.cwd();

	const files = process.argv.slice(2).filter((file) => {
		return file.endsWith('.ts') || file.endsWith('.vue');
	});

	const pipeline = new FormatPipeline({ sourceFiles: new NodeSourceFiles(), processRunner: new NodeProcessRunner() });

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
			diagnostics.push(formatError(failure.file, error));
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
