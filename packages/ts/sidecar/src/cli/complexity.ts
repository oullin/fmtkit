import { pathToFileURL } from 'node:url';
import { CompositionRoot } from '#sidecar/cli/composition-root';

/**
 * Run the complexity-scan entrypoint and map its exit code to the process status.
 *
 * @returns Nothing after running the command and setting the process status.
 */
export async function main(): Promise<void> {
	process.exitCode = await CompositionRoot.production()
		.complexityCommand()
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
