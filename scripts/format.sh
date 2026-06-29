#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/env.sh"

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
oxfmt_bin="${OXFMT_BIN:-packages/devx/node_modules/.bin/oxfmt}"
tsx_bin="${TSX_BIN:-packages/devx/node_modules/.bin/tsx}"
go_bin="${GO_BIN:-go}"

declare -a args=("$@")
declare -a go_fmt_args=()

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
	go_fmt_args+=("$(to_repo_path "$raw_arg")")
done

sources_workdir="$GO_WORKDIR"

if [[ "$sources_workdir" != /* ]]; then
	sources_workdir="$repo_root/$sources_workdir"
fi

ensure_storage_layout
"$go_bin" -C "$GO_WORKDIR" run "$CMD" format --cwd "$repo_root" "${go_fmt_args[@]}"

(
	cd "$repo_root"
	GO_BIN="$go_bin" \
		GO_FMT_SUPPORT_DIR="$repo_root/packages/devx" \
		GO_FMT_SOURCES_GO_WORKDIR="$sources_workdir" \
		GO_FMT_SOURCES_CWD="$repo_root" \
		GO_FMT_BLANK_LINES_SCRIPT="$repo_root/packages/devx/scripts/blank-lines.ts" \
		GO_FMT_FLUENT_CHAINS_SCRIPT="$repo_root/packages/devx/scripts/fluent-chains.ts" \
		GO_FMT_VALIDATE_SYNTAX_SCRIPT="$repo_root/packages/devx/scripts/validate-syntax.ts" \
		TSX_BIN="$tsx_bin" \
		OXFMT_BIN="$oxfmt_bin" \
		"$repo_root/cmd/fmt-ts-files.sh" "${go_fmt_args[@]}"
)
