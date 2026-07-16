#!/usr/bin/env bash
set -euo pipefail

# Runs this repository through fmtkit's own binary — the same Go orchestrator and
# bun-compiled TS sidecar a release carries. The host toolchain assets are staged
# on demand and reused until their sources change, so the inner loop stays a
# plain `go run`; the embedded-asset path releases use is covered separately by
# test-binary-smoke.sh.
#
# usage: fmtkit.sh <format|format-all|check|go|ts|lint|version|help> [args...]

source "$(dirname "${BASH_SOURCE[0]}")/env.sh"
source "${REPO_ROOT}/infra/scripts/host-target.sh"

support_dir="${REPO_ROOT}/packages/driver/internal/embedded/bin/$(host_target)"
sidecar="${support_dir}/fmtkit-ts-sidecar"

# The sidecar is stale once anything it is compiled from outdates it: the support
# scripts, the tool pins, or the configs staged alongside it.
sidecar_is_stale() {
	[[ -x "$sidecar" ]] || return 0

	local newer

	newer="$(find \
		"${REPO_ROOT}/packages/devx/scripts" \
		"${REPO_ROOT}/packages/devx/package.json" \
		"${REPO_ROOT}/.oxfmtrc.json" \
		"${REPO_ROOT}/.oxlintrc.json" \
		-newer "$sidecar" -print 2>/dev/null | head -n 1)"

	[[ -n "$newer" ]]
}

if sidecar_is_stale; then
	"${REPO_ROOT}/infra/scripts/release/stage-ts-assets.sh" host
fi

ensure_storage_layout

cd "$REPO_ROOT"

FMTKIT_SUPPORT_DIR="$support_dir" exec "${GO_BIN:-go}" run ./packages/driver/cmd/fmtkit "$@"
