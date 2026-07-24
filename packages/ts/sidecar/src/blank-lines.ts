import { pathToFileURL } from 'node:url';
import { FormatPipeline } from '#sidecar/format-pipeline';
import { PassCliDto } from '#sidecar/pass-cli-dto';
import { NodeProcessRunner } from '#sidecar/io/process-runner';
import { NodeSourceFiles } from '#sidecar/io/source-files';

async function main(): Promise<void> {
	const cwd = process.cwd();
	const options = PassCliDto.parse(process.argv.slice(2));
	const files = [...options.files];
	const { mode } = options;
	const pipeline = new FormatPipeline({ sourceFiles: new NodeSourceFiles(), processRunner: new NodeProcessRunner() });

	const outcomes = await pipeline.runPass('blank-lines', files, mode, (file, passMode) => {
		return pipeline.formatFile(file, passMode);
	});

	let changedCount = 0;

	for (const outcome of outcomes) {
		if (outcome.error?._tag === 'SourceFileUnreadable' && outcome.error.isNotFound()) {
			console.warn(`[blank-lines] path not found, skipping: ${outcome.file}`);

			continue;
		}

		if (outcome.error) {
			console.error(outcome.error);
			process.exitCode = 1;

			return;
		}

		if (!outcome.changed) {
			continue;
		}

		changedCount++;
		console.log(`[blank-lines] ${mode === 'check' ? 'would change' : 'updated'} ${outcome.file}`);
	}

	if (mode === 'check' && changedCount > 0) {
		console.error(`[blank-lines] ${changedCount} file(s) need blank-line edits. Run "pnpm format" to fix.`);
		process.exitCode = 1;

		return;
	}

	console.log(`[blank-lines] processed ${files.length} file(s) in ${cwd}, ${changedCount} ${mode === 'check' ? 'would change' : 'changed'}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	main().catch((error: unknown) => {
		console.error(error);
		process.exitCode = 1;
	});
}
