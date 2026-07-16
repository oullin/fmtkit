#!/usr/bin/env bash
set -euo pipefail

# Builds the self-contained TS toolchain assets embedded into the `fmtkit`
# release binary (see packages/driver/internal/tsruntime):
#
#   - fmtkit-ts-sidecar  bun-compiled bundle of packages/devx/scripts/sidecar.ts
#   - oxc-parser.node    napi binding, loaded via NAPI_RS_NATIVE_LIBRARY_PATH
#   - oxfmt.node         napi binding for the oxfmt CLI
#   - oxlint.node        napi binding for the oxlint CLI
#   - .oxfmtrc.json      repo-root config, the default for projects without one
#   - .oxlintrc.json     repo-root config, the default for projects without one
#
# Output lands in packages/driver/internal/embedded/bin/<target>/, next to the
# package that embeds it: go:embed cannot reach outside its own directory.
#
# Tool versions come from packages/devx/package.json devDependencies. Requires
# bash, node, npm, and bun.
#
# usage: stage-ts-assets.sh <all|host|goos_goarch...>

usage() {
	printf 'usage: %s <all|host|goos_goarch...>\n' "${0##*/}" >&2
	printf '  supported targets: darwin_arm64 darwin_amd64 linux_arm64 linux_amd64\n' >&2
}

if [[ $# -eq 0 ]]; then
	usage
	exit 2
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
dist="${FMTKIT_TS_ASSET_DIR:-${root}/packages/driver/internal/embedded/bin}"

source "${root}/infra/scripts/host-target.sh"

all_targets=(darwin_arm64 darwin_amd64 linux_arm64 linux_amd64)

bun_target() {
	case "$1" in
		darwin_arm64) printf 'bun-darwin-arm64' ;;
		darwin_amd64) printf 'bun-darwin-x64' ;;
		linux_arm64) printf 'bun-linux-arm64' ;;
		linux_amd64) printf 'bun-linux-x64' ;;
		*) return 1 ;;
	esac
}

binding_suffix() {
	case "$1" in
		darwin_arm64) printf 'darwin-arm64' ;;
		darwin_amd64) printf 'darwin-x64' ;;
		linux_arm64) printf 'linux-arm64-gnu' ;;
		linux_amd64) printf 'linux-x64-gnu' ;;
		*) return 1 ;;
	esac
}

declare -a targets=()

for arg in "$@"; do
	case "${arg}" in
		all)
			targets=("${all_targets[@]}")
			;;
		host)
			targets+=("$(host_target)")
			;;
		*)
			if ! bun_target "${arg}" >/dev/null; then
				printf 'unknown target: %s\n' "${arg}" >&2
				usage
				exit 2
			fi

			targets+=("${arg}")
			;;
	esac
done

for tool in node npm bun; do
	if ! command -v "${tool}" >/dev/null; then
		printf 'stage-ts-assets: %s is required on PATH\n' "${tool}" >&2
		exit 1
	fi
done

pin() {
	node -e "const p = require('${root}/packages/devx/package.json'); const v = p.devDependencies['$1']; if (!v) { throw new Error('no pin for $1'); } console.log(v);"
}

oxfmt_pin="$(pin oxfmt)"
oxlint_pin="$(pin oxlint)"
oxc_parser_pin="$(pin oxc-parser)"

workdir="$(mktemp -d)"
trap 'rm -rf "${workdir}"' EXIT

# Mirror the layout sidecar.ts expects in the repo: the scripts next to their
# package.json (for the #devx imports map) with node_modules one level up.
mkdir -p "${workdir}/scripts"

for script in "${root}"/packages/devx/scripts/*.ts; do
	case "${script##*/}" in
		*.test.ts) continue ;;
	esac

	cp "${script}" "${workdir}/scripts/"
done

cp "${root}/packages/devx/scripts/package.json" "${workdir}/scripts/package.json"

(
	cd "${workdir}"

	npm install --no-save --no-audit --no-fund \
		"oxfmt@${oxfmt_pin}" \
		"oxlint@${oxlint_pin}" \
		"oxc-parser@${oxc_parser_pin}" >/dev/null
)

# The napi bindings stay external: every target loads them from files staged
# next to the sidecar through NAPI_RS_NATIVE_LIBRARY_PATH, which keeps the JS
# bundle platform-independent. oxfmt's optional prettier plugins are external
# too; they are not installed by any fmtkit distribution channel.
#
# vite-plus is an optional peer of oxfmt/oxlint that both lazily import to read
# a Vite+ config file. Upstream ships the specifier external for the same
# reason. The sidecar bundles .oxfmtrc.json/.oxlintrc.json, so that path is
# unreachable here; a project that does keep its config in vite.config.ts has
# its own vite-plus for the import to resolve against.
bun_externals=(
	--external '@oxc-parser/binding-*'
	--external '@oxfmt/binding-*'
	--external '@oxlint/binding-*'
	--external '@prettier/*'
	--external 'prettier-plugin-*'
	--external '@shopify/prettier-plugin-liquid'
	--external '@zackad/prettier-plugin-twig'
	--external 'vite-plus'
)

fetch_binding() {
	local package="$1"
	local file="$2"
	local destination="$3"
	local pack_dir

	pack_dir="$(mktemp -d "${workdir}/pack.XXXXXX")"

	(
		cd "${pack_dir}"

		npm pack --silent "${package}" >/dev/null
		tar -xzf ./*.tgz
	)

	install -m 0644 "${pack_dir}/package/${file}" "${destination}"
	rm -rf "${pack_dir}"
}

for target in "${targets[@]}"; do
	suffix="$(binding_suffix "${target}")"
	out="${dist}/${target}"

	printf '==> staging TS assets for %s\n' "${target}" >&2

	rm -rf "${out}"
	mkdir -p "${out}"

	# Run bun from the throwaway workdir: bun drops *.bun-build scratch files
	# into the invoking directory, and a dirty repo would fail the release's
	# git state check.
	(
		cd "${workdir}"

		bun build --compile \
			--target "$(bun_target "${target}")" \
			--outfile "${out}/fmtkit-ts-sidecar" \
			"${bun_externals[@]}" \
			"${workdir}/scripts/sidecar.ts" >/dev/null
	)

	fetch_binding "@oxc-parser/binding-${suffix}@${oxc_parser_pin}" "parser.${suffix}.node" "${out}/oxc-parser.node"
	fetch_binding "@oxfmt/binding-${suffix}@${oxfmt_pin}" "oxfmt.${suffix}.node" "${out}/oxfmt.node"
	fetch_binding "@oxlint/binding-${suffix}@${oxlint_pin}" "oxlint.${suffix}.node" "${out}/oxlint.node"

	# The repo root configs are the single source of truth: they double as the
	# configs this repo formats itself with, and tsruntime extracts these
	# copies as the fallback for projects that carry none.
	install -m 0644 "${root}/.oxfmtrc.json" "${out}/.oxfmtrc.json"
	install -m 0644 "${root}/.oxlintrc.json" "${out}/.oxlintrc.json"

	printf '==> staged %s\n' "${out}" >&2
done
