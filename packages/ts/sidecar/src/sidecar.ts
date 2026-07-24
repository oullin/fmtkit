/**
 * Entry point for the self-contained TS toolchain sidecar bundled into the
 * `fmtkit` release binary via `bun build --compile` (see
 * infra/scripts/release/stage-ts-assets.sh).
 *
 * One executable multiplexes the three Node-based tools so the Bun runtime is
 * only shipped once. The napi bindings (oxc-parser, oxfmt, oxlint) are NOT
 * bundled; they are extracted next to this executable and loaded through the
 * napi-rs NAPI_RS_NATIVE_LIBRARY_PATH override.
 */
import { dirname, join } from 'node:path';
import { z } from 'zod';

type Mode = 'pipeline' | 'oxfmt' | 'oxlint';

const ModeSchema = z.enum(['pipeline', 'oxfmt', 'oxlint']);

/** Immutable sidecar mode selected from CLI and environment boundaries. */
class SidecarRuntimeDto {
	/** The selected sidecar mode, or `undefined` for usage output. */
	readonly mode: Mode | undefined;
	/** Whether the selected mode came from the positional CLI argument. */
	readonly consumeArgument: boolean;

	static readonly #schema = z.object({
		argument: ModeSchema.optional().catch(undefined),
		environment: ModeSchema.optional().catch(undefined),
	});

	private constructor(mode: Mode | undefined, consumeArgument: boolean) {
		this.mode = mode;
		this.consumeArgument = consumeArgument;

		Object.freeze(this);
	}

	/**
	 * Parse the sidecar mode inputs once, preferring the CLI argument.
	 *
	 * @param input - The untrusted CLI and environment mode values.
	 * @returns An immutable runtime selection.
	 */
	static from(input: unknown): SidecarRuntimeDto {
		const parsed = SidecarRuntimeDto.#schema.parse(input);

		return new SidecarRuntimeDto(parsed.argument ?? parsed.environment, parsed.argument !== undefined);
	}
}

const here = dirname(process.execPath);

const bindings: Record<Mode, string> = {
	pipeline: join(here, 'oxc-parser.node'),
	oxfmt: join(here, 'oxfmt.node'),
	oxlint: join(here, 'oxlint.node'),
};

const runtime = SidecarRuntimeDto.from({
	argument: process.argv[2],
	environment: process.env.FMTKIT_SIDECAR_MODE,
});

const mode = runtime.mode;

if (runtime.consumeArgument) {
	process.argv.splice(2, 1);
}

switch (mode) {
	case 'pipeline': {
		process.env.NAPI_RS_NATIVE_LIBRARY_PATH = bindings.pipeline;
		process.env.FMTKIT_SIDECAR_MODE = 'oxfmt';

		// Every bundled module shares this executable's import.meta.url, which
		// matches argv[1] and would fire each script's run-as-main guard on
		// import; blank argv[1] so only the explicit main() call below runs.
		process.argv[1] = '';

		const { main } = await import('#sidecar/cli/format-all');

		await main();

		break;
	}

	case 'oxfmt':
		process.env.NAPI_RS_NATIVE_LIBRARY_PATH = bindings.oxfmt;

		await import('../node_modules/oxfmt/dist/cli.js');

		break;

	case 'oxlint':
		process.env.NAPI_RS_NATIVE_LIBRARY_PATH = bindings.oxlint;

		// @ts-expect-error -- the oxlint CLI entry ships without declarations.
		await import('../node_modules/oxlint/dist/cli.js');

		break;

	default:
		console.error('usage: fmtkit-ts-sidecar <pipeline|oxfmt|oxlint> [args...]');
		process.exit(2);
}
