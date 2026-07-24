/** A runnable sidecar CLI command that maps parsed arguments to an exit code. */
export interface CliCommand {
	/**
	 * Run the command over already-sliced CLI arguments.
	 *
	 * @param argv - Arguments after the executable and script path.
	 * @returns The process exit code; the command never calls `process.exit`.
	 */
	run(argv: readonly string[]): Promise<number>;
}
