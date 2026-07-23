import { pathToFileURL } from 'node:url';
import { FormatPipeline } from '#sidecar/pipeline/format-pipeline';
import { NodeProcessRunner } from '#sidecar/io/process-runner';
import { NodeSourceFiles } from '#sidecar/io/source-files';
import { PassCliDto } from '#sidecar/pass-cli-dto';
import { PipelineFactory } from '#sidecar/pipeline/pipeline-factory';
import { SourceFileEditor } from '#sidecar/pipeline/source-file-editor';

/**
 * Run the standalone fluent-chain formatter entrypoint.
 *
 * @returns Nothing after reporting outcomes and setting the process status.
 */
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

	const outcomes = await pipeline.runPass(factory.fluentFormatter(), files, mode);

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

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	main().catch((err: unknown) => {
		console.error(err);
		process.exit(1);
	});
}
