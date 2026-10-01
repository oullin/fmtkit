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
	try {
		await main();
	} catch (cause) {
		process.stderr.write(`${String(cause)}\n`);
		process.exitCode = 1;
	}
}
