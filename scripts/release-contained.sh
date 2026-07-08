#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/env.sh"

dist_dir_path="$(canonical_path "$DIST_DIR")"
release_platforms="${RELEASE_PLATFORMS:-$(go -C "$GO_WORKDIR" env GOOS)/$(go -C "$GO_WORKDIR" env GOARCH)}"

ensure_storage_layout
mkdir -p "$dist_dir_path"

for platform in $release_platforms; do
	if [[ "$platform" != */* ]]; then
		printf 'invalid release platform: %s\n' "$platform" >&2
		exit 1
	fi

	goos="${platform%/*}"
	goarch="${platform#*/}"
	archive="$REPO_ROOT/packages/driver/internal/full/assets/runtime-${goos}-${goarch}.tar.gz"

	if [[ ! -f "$archive" ]]; then
		printf 'missing contained runtime archive: %s\n' "$archive" >&2
		printf 'run RUNTIME_GOOS=%s RUNTIME_GOARCH=%s ./scripts/package-contained-runtime.sh on the matching platform, or provide the archive before release.\n' "$goos" "$goarch" >&2
		exit 1
	fi

	output="${dist_dir_path}/fmt-all-${goos}-${goarch}"
	printf 'Building contained fmt-all (%s/%s)...\n' "$goos" "$goarch"

	CGO_ENABLED="$CGO_ENABLED" GOOS="$goos" GOARCH="$goarch" \
		go -C "$GO_WORKDIR" build -trimpath -ldflags "-s -w -X main.version=$VERSION" -o "$output" ./cmd/fmt-all

	chmod +x "$output"
done
