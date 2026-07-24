import { pathToFileURL } from 'node:url';
import { CompositionRoot } from '#sidecar/cli/composition-root';

/**
 * Run the full formatting CLI and map its exit code to the process status.
 *
 * @returns Nothing after running the command and setting the process status.
 */
export async function main(): Promise<void> {
	process.exitCode = await CompositionRoot.production()
		.formatAllCommand()
		.run(process.argv.slice(2));
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	main().catch((error: unknown) => {
		console.error(error);
		process.exitCode = 1;
	});
}
