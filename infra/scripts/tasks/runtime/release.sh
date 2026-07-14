#!/usr/bin/env bash
set -euo pipefail

task_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
source "$task_dir/env.sh"
source "$task_dir/runtime/common.sh"
source "$task_dir/runtime/platform.sh"
source "$task_dir/runtime/archive.sh"

require_no_extra_args "$@"
native_goos="$(go -C "$GO_WORKDIR" env GOOS)"
native_goarch="$(go -C "$GO_WORKDIR" env GOARCH)"
release_platforms="$(trim "${RELEASE_PLATFORMS:-$native_goos/$native_goarch}")"
read -r -a platforms <<<"$release_platforms"
if ((${#platforms[@]} != 1)); then
	printf 'contained release must build exactly one native platform\n' >&2
	exit 1
fi

platform="${platforms[0]}"
if [[ ! "$platform" =~ ^[a-z0-9]+/[a-z0-9]+$ ]]; then
	printf 'invalid release platform: %s\n' "$platform" >&2
	exit 1
fi
goos="${platform%/*}"
goarch="${platform#*/}"
validate_runtime_platform "$goos" "$goarch"
if [[ "$goos/$goarch" != "$native_goos/$native_goarch" ]]; then
	printf 'contained release must run natively; requested %s, Go is %s/%s\n' "$platform" "$native_goos" "$native_goarch" >&2
	exit 1
fi

ensure_storage_layout
dist_dir_path="$(canonical_path "$DIST_DIR")"
mkdir -p "$dist_dir_path"
archive="$(runtime_archive_path "$goos" "$goarch")"
manifest="$archive.manifest.json"
validate_archive_inputs "$archive" "$manifest" "$goos" "$goarch"

output="$dist_dir_path/fmtkit-${goos}-${goarch}"
[[ "$(basename "$output")" == "fmtkit-${goos}-${goarch}" ]] || {
	printf 'contained binary name does not match requested platform: %s\n' "$output" >&2
	exit 1
}
printf 'Building contained fmtkit (%s/%s)...\n' "$goos" "$goarch"
CGO_ENABLED="$CGO_ENABLED" GOOS="$goos" GOARCH="$goarch" \
	go -C "$GO_WORKDIR" build -trimpath -ldflags "-s -w -X main.version=$VERSION" -o "$output" ./packages/driver/cmd/fmtkit
chmod +x "$output"
