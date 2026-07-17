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

const modes = ['pipeline', 'oxfmt', 'oxlint'] as const;

type Mode = (typeof modes)[number];

const here = dirname(process.execPath);

const bindings: Record<Mode, string> = {
	pipeline: join(here, 'oxc-parser.node'),
	oxfmt: join(here, 'oxfmt.node'),
	oxlint: join(here, 'oxlint.node'),
};

let mode: Mode | undefined;

if (modes.includes(process.argv[2] as Mode)) {
	mode = process.argv[2] as Mode;

	process.argv.splice(2, 1);
} else if (modes.includes(process.env.FMTKIT_SIDECAR_MODE as Mode)) {
	// The pipeline spawns this executable as its `--oxfmt-bin` with oxfmt's own
	// argv, so the mode has to travel through the environment instead.
	mode = process.env.FMTKIT_SIDECAR_MODE as Mode;
}

switch (mode) {
	case 'pipeline': {
		process.env.NAPI_RS_NATIVE_LIBRARY_PATH = bindings.pipeline;
		process.env.FMTKIT_SIDECAR_MODE = 'oxfmt';

		// Every bundled module shares this executable's import.meta.url, which
		// matches argv[1] and would fire each script's run-as-main guard on
		// import; blank argv[1] so only the explicit main() call below runs.
		process.argv[1] = '';

		const { main } = await import('#sidecar/format-all');

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
