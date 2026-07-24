import { pathToFileURL } from 'node:url';
import { FormatPipeline } from '#sidecar/pipeline/format-pipeline';
import { NodeProcessRunner } from '#sidecar/io/process-runner';
import { NodeSourceFiles } from '#sidecar/io/source-files';
import { PassCliDto } from '#sidecar/pass-cli-dto';
import { PipelineFactory } from '#sidecar/pipeline/pipeline-factory';
import { SourceFileEditor } from '#sidecar/pipeline/source-file-editor';

async function main(): Promise<void> {
	const cwd = process.cwd();
	const options = PassCliDto.parse(process.argv.slice(2));
	const files = [...options.files];
	const { mode } = options;
	const factory = PipelineFactory.create();
	const sourceFiles = new NodeSourceFiles();

	const pipeline = new FormatPipeline({
		editor: new SourceFileEditor({ sourceFiles }),
		processRunner: new NodeProcessRunner(),
		validator: factory.syntaxValidator(sourceFiles),
	});

	const outcomes = await pipeline.runPass(factory.segmentFormatter(), files, mode);

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
