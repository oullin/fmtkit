#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/env.sh"

dist_dir_path="$(canonical_path "$DIST_DIR")"
release_platforms="${RELEASE_PLATFORMS:-$(go -C "$GO_WORKDIR" env GOOS)/$(go -C "$GO_WORKDIR" env GOARCH)}"

ensure_storage_layout
mkdir -p "$dist_dir_path"


read -r -a platforms <<<"$release_platforms"

for platform in "${platforms[@]}"; do
	if [[ ! "$platform" =~ ^[a-z0-9]+/[a-z0-9]+$ ]]; then
		printf 'invalid release platform: %s\n' "$platform" >&2
		exit 1
	fi

	goos="${platform%/*}"
	goarch="${platform#*/}"
	archive="$REPO_ROOT/packages/driver/internal/full/assets/runtime-${goos}-${goarch}.tar.gz"
	manifest="$archive.manifest.json"

	if [[ ! -f "$archive" ]]; then
		printf 'missing contained runtime archive: %s\n' "$archive" >&2
		printf 'run RUNTIME_GOOS=%s RUNTIME_GOARCH=%s ./infra/scripts/tasks/package-contained-runtime.sh on the matching platform, or provide the archive before release.\n' "$goos" "$goarch" >&2
		exit 1
	fi

	if [[ ! -f "$manifest" ]]; then
		printf 'missing runtime manifest: %s\n' "$manifest" >&2
		exit 1
	fi

	gzip -t "$archive"

	for member in bin/node bin/fmt-ts bin/fmt-lint bin/tsx bin/oxfmt bin/oxlint go/bin/go support/scripts/format-all.ts; do
		if ! tar -tzf "$archive" | grep -Fxq "$member"; then
			printf 'runtime archive %s is missing required member %s\n' "$archive" "$member" >&2
			exit 1
		fi
	done

	if tar -tzf "$archive" | awk -F/ 'NF == 0 || ($1 != "bin" && $1 != "go" && $1 != "lib" && $1 != "support") { exit 1 }'; then :; else
		printf 'runtime archive contains an unexpected top-level path: %s\n' "$archive" >&2
		exit 1
	fi

	output="${dist_dir_path}/fmt-all-${goos}-${goarch}"
	printf 'Building contained fmt-all (%s/%s)...\n' "$goos" "$goarch"

	CGO_ENABLED="$CGO_ENABLED" GOOS="$goos" GOARCH="$goarch" \
		go -C "$GO_WORKDIR" build -trimpath -ldflags "-s -w -X main.version=$VERSION" -o "$output" ./packages/driver/cmd/fmt-all

	chmod +x "$output"
done
