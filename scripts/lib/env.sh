#!/usr/bin/env bash

# Shared environment for the repository scripts. Every artifact they produce
# lives under storage/; ensure_storage_layout asserts it.

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
export REPO_ROOT="${REPO_ROOT:-$(cd "${script_dir}/../.." && pwd -P)}"

# The version fmtkit reports (CARGO_PKG_VERSION), which the Go helper must be
# stamped with for a release build's handshake to accept it.
export VERSION="${VERSION:-$(sed -n '/^\[workspace.package\]/,/^\[/ s/^version = "\(.*\)"$/\1/p' "${REPO_ROOT}/Cargo.toml")}"

export STORAGE_DIR="${STORAGE_DIR:-${REPO_ROOT}/storage}"
export CACHE_DIR="${CACHE_DIR:-${STORAGE_DIR}/.cache}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${STORAGE_DIR}/target}"
export GO_HELPER_DIR="${GO_HELPER_DIR:-storage/go-helper}"
export DIST_TEST_DIR="${DIST_TEST_DIR:-storage/dist-test}"
export GOCACHE="${GOCACHE:-${CACHE_DIR}/go-build}"
export GOPATH="${GOPATH:-${CACHE_DIR}/gopath}"
export GOMODCACHE="${GOMODCACHE:-${GOPATH}/pkg/mod}"

repo_path() {
	local path="$1"

	case "$path" in
		/*)
			printf '%s\n' "$path"
			;;
		*)
			printf '%s\n' "${REPO_ROOT}/${path}"
			;;
	esac
}

canonical_path() {
	local path="$1"
	local dir
	local base

	path="${path%/}"
	dir="$(dirname "$path")"
	base="$(basename "$path")"
	dir="$(repo_path "$dir")"
	mkdir -p "$dir"
	printf '%s/%s\n' "$(cd "$dir" && pwd -P)" "$base"
}

assert_under_storage() {
	local label="$1"
	local path="$2"
	local resolved

	resolved="$(canonical_path "$path")"

	case "$resolved" in
		"${STORAGE_DIR}" | "${STORAGE_DIR}"/*)
			;;
		*)
			printf '%s must resolve under %s, got %s\n' "$label" "$STORAGE_DIR" "$resolved" >&2
			exit 1
			;;
	esac
}

assert_no_legacy_artifacts() {
	local forbidden

	for forbidden in \
		"${REPO_ROOT}/.gocache" \
		"${REPO_ROOT}/.gopath" \
		"${REPO_ROOT}/bin" \
		"${REPO_ROOT}/dist" \
		"${REPO_ROOT}/dist-test"; do
		if [[ -e "$forbidden" ]]; then
			printf 'legacy repo-root artifact path is not allowed: %s\n' "$forbidden" >&2
			exit 1
		fi
	done
}

ensure_storage_layout() {
	local dir

	for dir in CARGO_TARGET_DIR GO_HELPER_DIR DIST_TEST_DIR GOCACHE GOPATH GOMODCACHE; do
		assert_under_storage "$dir" "${!dir}"
		mkdir -p "$(canonical_path "${!dir}")"
	done

	mkdir -p "${CACHE_DIR}"
	assert_no_legacy_artifacts
}
