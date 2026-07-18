/**
 * Rewrites oxfmt to format embedded code in-process, so the `bun build
 * --compile` binary stops hanging on Vue/markdown/HTML files. Invoked by
 * stage-ts-assets.sh between installing oxfmt and bundling it; see
 * `OxfmtCliPatcher` for why the rewrite exists.
 *
 * usage: node patch-oxfmt-inprocess.ts <path/to/oxfmt/dist>
 */
import { NodeTextFiles, OxfmtCliPatcher, PatchCliDto, isErr } from '#oxfmt-inprocess';

const NAME = 'patch-oxfmt-inprocess';
const USAGE_EXIT_CODE = 2;

function main(): void {
	const options = PatchCliDto.parse(process.argv.slice(2));

	if (options === null) {
		process.stderr.write(`usage: node ${NAME}.ts <path/to/oxfmt/dist>\n`);
		process.exit(USAGE_EXIT_CODE);
	}

	const patched = new OxfmtCliPatcher(new NodeTextFiles()).patch(options.distDir);

	if (isErr(patched)) {
		process.stderr.write(`${NAME}: ${patched.error.message}\n`);
		process.exit(1);
	}

	process.stderr.write(`${NAME}: rewrote ${patched.value.cliPath} to format embedded code in-process (api module: ${patched.value.apiModuleSpecifier})\n`);
}

main();
