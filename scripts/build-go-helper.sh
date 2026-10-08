#!/usr/bin/env bash
# Builds fmtkit-go-helper for one Rust target triple (default: this machine)
# into storage/go-helper/, stamped with the fmtkit version so the handshake
# with a release fmtkit binary matches.
set -euo pipefail

source "$(dirname "$0")/lib/env.sh"

if (($# > 1)); then
	printf 'build-go-helper: expected at most one target, got: %s\n' "$*" >&2
	exit 2
fi

target="${1:-$(rustc -vV | sed -n 's/^host: //p')}"

case "$target" in
	aarch64-apple-darwin) goos=darwin goarch=arm64 ;;
	x86_64-apple-darwin) goos=darwin goarch=amd64 ;;
	aarch64-unknown-linux-gnu) goos=linux goarch=arm64 ;;
	x86_64-unknown-linux-gnu) goos=linux goarch=amd64 ;;
	*)
		printf 'build-go-helper: unsupported target %s\n' "$target" >&2
		exit 2
		;;
esac

output="$(canonical_path "$GO_HELPER_DIR")/fmtkit-go-helper"

CGO_ENABLED=0 GOOS="$goos" GOARCH="$goarch" \
	go -C "${REPO_ROOT}/go/helper" build -trimpath -ldflags "-s -w -X main.version=${VERSION}" -o "$output" .

printf 'built %s (%s)\n' "$output" "$target"
