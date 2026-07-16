#!/usr/bin/env bash
set -euo pipefail

# Formats this repository with fmtkit's own binary. Paths are resolved against
# the repository root rather than the invoking directory, so `format.sh .` means
# the whole repo no matter where it is run from.

source "$(dirname "${BASH_SOURCE[0]}")/env.sh"

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
		-*)
			# A step or output flag (--ts, --go, --quiet): pass it through as-is.
			printf '%s\n' "$arg"
			;;
		.)
			printf '%s\n' "$REPO_ROOT"
			;;
		./*)
			printf '%s\n' "$REPO_ROOT/${arg#./}"
			;;
		/*)
			printf '%s\n' "$arg"
			;;
		*)
			printf '%s\n' "$REPO_ROOT/$arg"
			;;
	esac
}

for raw_arg in "${args[@]}"; do
	fmtkit_args+=("$(to_repo_path "$raw_arg")")
done

exec "$(dirname "${BASH_SOURCE[0]}")/fmtkit.sh" format "${fmtkit_args[@]}"
