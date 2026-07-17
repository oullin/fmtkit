#!/usr/bin/env bash

export APP="${APP:-fmtkit-go}"
export CMD="${CMD:-./driver/cmd/fmtkit-go}"
export CGO_ENABLED="${CGO_ENABLED:-0}"

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
export REPO_ROOT="${REPO_ROOT:-$(cd "${script_dir}/../.." && pwd -P)}"
export GO_WORKDIR="${GO_WORKDIR:-${REPO_ROOT}/packages/go}"
export VERSION="${VERSION:-$(git -C "$REPO_ROOT" describe --tags --always --dirty 2>/dev/null || echo dev)}"
export STORAGE_DIR="${STORAGE_DIR:-${REPO_ROOT}/storage}"
export CACHE_DIR="${CACHE_DIR:-${STORAGE_DIR}/.cache}"
export BUILD_DIR="${BUILD_DIR:-storage/bin}"
export BIN="${BIN:-${BUILD_DIR}/${APP}}"
export DIST_DIR="${DIST_DIR:-storage/dist}"
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
		"${REPO_ROOT}/.turbo" \
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
	assert_under_storage "BUILD_DIR" "$BUILD_DIR"
	assert_under_storage "BIN" "$BIN"
	assert_under_storage "DIST_DIR" "$DIST_DIR"
	assert_under_storage "DIST_TEST_DIR" "$DIST_TEST_DIR"
	assert_under_storage "GOCACHE" "$GOCACHE"
	assert_under_storage "GOPATH" "$GOPATH"
	assert_under_storage "GOMODCACHE" "$GOMODCACHE"

	mkdir -p \
		"${STORAGE_DIR}" \
		"${CACHE_DIR}" \
		"$(canonical_path "$BUILD_DIR")" \
		"$(canonical_path "$DIST_DIR")" \
		"$(canonical_path "$DIST_TEST_DIR")" \
		"$(canonical_path "$GOCACHE")" \
		"$(canonical_path "$GOPATH")" \
		"$(canonical_path "$GOMODCACHE")" \
		"$(dirname "$(canonical_path "$BIN")")"

	assert_no_legacy_artifacts
}
