#!/usr/bin/env bash

trim() {
	local value="$1"
	value="${value#"${value%%[![:space:]]*}"}"
	value="${value%"${value##*[![:space:]]}"}"
	printf '%s\n' "$value"
}

require_nonblank() {
	local label="$1"
	local value

	value="$(trim "$2")"
	if [[ -z "$value" ]]; then
		printf '%s must be nonblank\n' "$label" >&2
		return 1
	fi

	printf '%s\n' "$value"
}

require_no_extra_args() {
	if (( $# != 0 )); then
		printf 'unexpected argument: %s\n' "$1" >&2
		return 1
	fi
}

runtime_assets_dir() {
	printf '%s\n' "$REPO_ROOT/packages/runtimex/assets"
}

portable_sha256() {
	local path="$1"
	local digest

	if command -v sha256sum >/dev/null 2>&1; then
		digest="$(sha256sum "$path" | awk '{print $1}')"
	else
		digest="$(shasum -a 256 "$path" | awk '{print $1}')"
	fi

	printf '%s  %s\n' "$digest" "$(basename "$path")"
}
