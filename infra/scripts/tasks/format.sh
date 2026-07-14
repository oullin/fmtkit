#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/env.sh"

repo_root="$REPO_ROOT"
oxfmt_bin="${OXFMT_BIN:-packages/devx/node_modules/.bin/oxfmt}"
tsx_bin="${TSX_BIN:-packages/devx/node_modules/.bin/tsx}"
go_bin="${GO_BIN:-go}"

declare -a args=("$@")
declare -a fmtkit_args=()

if [[ "${args[0]:-}" == "--" ]]; then
	args=("${args[@]:1}")
fi

if [[ ${#args[@]} -eq 0 ]]; then
	args=(.)
fi

to_repo_path() {
	local arg="$1"

	case "$arg" in
		.)
			printf '%s\n' "$repo_root"
			;;
		./*)
			printf '%s\n' "$repo_root/${arg#./}"
			;;
		/*)
			printf '%s\n' "$arg"
			;;
		*)
			printf '%s\n' "$repo_root/$arg"
			;;
	esac
}

for raw_arg in "${args[@]}"; do
	fmtkit_args+=("$(to_repo_path "$raw_arg")")
done

sources_workdir="$GO_WORKDIR"

if [[ "$sources_workdir" != /* ]]; then
	sources_workdir="$repo_root/$sources_workdir"
fi

ensure_storage_layout
"$go_bin" -C "$GO_WORKDIR" run "$CMD" format --cwd "$repo_root" "${fmtkit_args[@]}"

(
	cd "$repo_root"
	GO_BIN="$go_bin" \
		FMTKIT_SUPPORT_DIR="$repo_root/packages/devx" \
		FMTKIT_SOURCES_GO_WORKDIR="$sources_workdir" \
		FMTKIT_SOURCES_CWD="$repo_root" \
		FMTKIT_FORMAT_ALL_SCRIPT="$repo_root/packages/devx/scripts/format-all.ts" \
		TSX_BIN="$tsx_bin" \
		OXFMT_BIN="$oxfmt_bin" \
		"$repo_root/infra/bin/fmtkit-ts-files" "${fmtkit_args[@]}"
)
