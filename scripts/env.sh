#!/usr/bin/env bash

export APP="${APP:-go-fmt}"
export GO_WORKDIR="${GO_WORKDIR:-packages/driver}"
export CMD="${CMD:-./cmd/fmt-go}"
export VERSION="${VERSION:-dev}"
export CGO_ENABLED="${CGO_ENABLED:-0}"

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export REPO_ROOT="${REPO_ROOT:-$(cd "${script_dir}/.." && pwd)}"
export STORAGE_DIR="${STORAGE_DIR:-${REPO_ROOT}/storage}"
export CACHE_DIR="${CACHE_DIR:-${STORAGE_DIR}/.cache}"
export BUILD_DIR="${BUILD_DIR:-storage/bin}"
export BIN="${BIN:-${BUILD_DIR}/${APP}}"
export DIST_DIR="${DIST_DIR:-storage/dist}"
export DIST_TEST_DIR="${DIST_TEST_DIR:-storage/dist-test}"
export GOCACHE="${GOCACHE:-${CACHE_DIR}/go-build}"
export GOPATH="${GOPATH:-${CACHE_DIR}/gopath}"
export GOMODCACHE="${GOMODCACHE:-${GOPATH}/pkg/mod}"
export TURBO_CACHE_DIR="${TURBO_CACHE_DIR:-${CACHE_DIR}/turbo}"

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

assert_no_package_turbo_dirs() {
	local leaked_turbo_dir

	leaked_turbo_dir="$(find "${REPO_ROOT}/packages" -type d -name .turbo -print -quit 2>/dev/null || true)"
	if [[ -n "$leaked_turbo_dir" ]]; then
		printf 'package-local turbo artifacts are not allowed outside %s: %s\n' "$TURBO_CACHE_DIR" "$leaked_turbo_dir" >&2
		exit 1
	fi
}

cleanup_package_turbo_logs() {
	local turbo_dir
	local relative_dir
	local target_dir

	while IFS= read -r turbo_dir; do
		relative_dir="${turbo_dir#${REPO_ROOT}/}"
		target_dir="$(canonical_path "${TURBO_CACHE_DIR}/logs/${relative_dir%/.turbo}")"
		mkdir -p "$target_dir"
		find "$turbo_dir" -maxdepth 1 -type f -exec mv {} "$target_dir"/ \;
		rm -rf "$turbo_dir"
	done < <(find "${REPO_ROOT}/packages" -type d -name .turbo -print 2>/dev/null)
}

ensure_storage_layout() {
	assert_under_storage "BUILD_DIR" "$BUILD_DIR"
	assert_under_storage "BIN" "$BIN"
	assert_under_storage "DIST_DIR" "$DIST_DIR"
	assert_under_storage "DIST_TEST_DIR" "$DIST_TEST_DIR"
	assert_under_storage "GOCACHE" "$GOCACHE"
	assert_under_storage "GOPATH" "$GOPATH"
	assert_under_storage "GOMODCACHE" "$GOMODCACHE"
	assert_under_storage "TURBO_CACHE_DIR" "$TURBO_CACHE_DIR"

	mkdir -p \
		"${STORAGE_DIR}" \
		"${CACHE_DIR}" \
		"$(canonical_path "$BUILD_DIR")" \
		"$(canonical_path "$DIST_DIR")" \
		"$(canonical_path "$DIST_TEST_DIR")" \
		"$(canonical_path "$GOCACHE")" \
		"$(canonical_path "$GOPATH")" \
		"$(canonical_path "$GOMODCACHE")" \
		"$(canonical_path "$TURBO_CACHE_DIR")" \
		"$(dirname "$(canonical_path "$BIN")")"

	assert_no_legacy_artifacts
}
