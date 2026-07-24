import { spawn } from 'node:child_process';
import { OxfmtRunFailed } from '#sidecar/kernel/errors';
import { err, ok } from '#sidecar/kernel/result';
import type { Result } from '#sidecar/kernel/result';

/** The process operation required to invoke oxfmt. */
export type ProcessRunner = {
	/**
	 * Run oxfmt with inherited standard streams.
	 *
	 * @param bin - The oxfmt executable.
	 * @param args - The arguments passed to oxfmt.
	 * @returns Nothing, or a typed process failure.
	 */
	run(bin: string, args: string[]): Promise<Result<void, OxfmtRunFailed>>;
};

/** Runs oxfmt as a Node child process with inherited standard streams. */
export class NodeProcessRunner implements ProcessRunner {
	/**
	 * Run oxfmt with inherited standard streams.
	 *
	 * @param bin - The oxfmt executable.
	 * @param args - The arguments passed to oxfmt.
	 * @returns Nothing, or `OxfmtRunFailed` carrying its status or spawn cause.
	 */
	run(bin: string, args: string[]): Promise<Result<void, OxfmtRunFailed>> {
		return new Promise((resolvePromise) => {
			const child = spawn(
				bin,
				args,
				{ stdio: 'inherit' },
			);

			child.once('error', (cause) => {
				resolvePromise(
					err(new OxfmtRunFailed(bin, null, null, cause)),
				);
			});

			child.once('exit', (code, signal) => {
				if (code === 0) {
					resolvePromise(
						ok(undefined),
					);
				} else {
					resolvePromise(
						err(new OxfmtRunFailed(bin, code, signal, undefined)),
					);
				}
			});
		});
	}
}
