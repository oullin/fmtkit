#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/env.sh"

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
oxfmt_bin="${OXFMT_BIN:-packages/devx/node_modules/.bin/oxfmt}"
tsx_bin="${TSX_BIN:-packages/devx/node_modules/.bin/tsx}"

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

ensure_storage_layout
go -C "$GO_WORKDIR" run "$CMD" format --cwd "$repo_root" "${go_fmt_args[@]}"

(cd "$repo_root" && "$tsx_bin" packages/devx/scripts/blank-lines.ts "${go_fmt_args[@]}")

git ls-files --cached --others --exclude-standard -z -- "${args[@]}" \
	| xargs -0 "$oxfmt_bin" --write --no-error-on-unmatched-pattern
