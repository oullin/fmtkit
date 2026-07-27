import { pathToFileURL } from 'node:url';
import { CompositionRoot } from '#sidecar/cli/composition-root';

/**
 * Run the standalone blank-lines segment formatter entrypoint.
 *
 * @returns Nothing after running the command and setting the process status.
 */
async function main(): Promise<void> {
	process.exitCode = await CompositionRoot.production()
		.segmentPassCommand()
		.run(process.argv.slice(2));
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	main().catch((error: unknown) => {
		console.error(error);
		process.exitCode = 1;
	});
}
