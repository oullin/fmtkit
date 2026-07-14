#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/env.sh"

build_dir="${BUILD_DIR}"
bin="${BIN}"
host_os="${HOST_OS:-$(go -C "$GO_WORKDIR" env GOOS)}"
host_arch="${HOST_ARCH:-$(go -C "$GO_WORKDIR" env GOARCH)}"
build_dir_path="$(canonical_path "$build_dir")"
bin_path="$(canonical_path "$bin")"

ensure_storage_layout
mkdir -p "$build_dir_path" "$(dirname "$bin_path")"

CGO_ENABLED="$CGO_ENABLED" GOOS="$host_os" GOARCH="$host_arch" \
	go -C "$GO_WORKDIR" build -trimpath -ldflags "-s -w -X main.version=$VERSION" -o "$bin_path" "$CMD"
chmod +x "$bin_path"
