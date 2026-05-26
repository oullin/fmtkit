#!/usr/bin/env bash
# go-fmt-host.sh — single-image, no-compose host wrapper for go-fmt.
#
# Every project on the host shares one selected image (GO_FMT_IMAGE, default
# ghcr.io/oullin/go-fmt:latest) and one named cache volume (GO_FMT_CACHE_VOLUME,
# default go-fmt-cache). Drop this script anywhere on $PATH (or symlink it as
# `go-fmt`) and invoke from any project root:
#
#     go-fmt-host.sh format .
#     go-fmt-host.sh format-all
#     go-fmt-host.sh go check ./pkg ./cmd
#     go-fmt-host.sh ts .
#
# Select a formatter flavor by exporting GO_FMT_IMAGE:
#
#     GO_FMT_IMAGE=ghcr.io/oullin/go-fmt:latest-full
#     GO_FMT_IMAGE=ghcr.io/oullin/go-fmt:latest-go
#     GO_FMT_IMAGE=ghcr.io/oullin/go-fmt:latest-node-ts
#
# Versioned flavor tags follow the same shape, for example v0.0.18-go,
# v0.0.18-node-ts, and v0.0.18-full.

set -euo pipefail

image="${GO_FMT_IMAGE:-ghcr.io/oullin/go-fmt:latest}"
cache_volume="${GO_FMT_CACHE_VOLUME:-go-fmt-cache}"
project_dir="${GO_FMT_PROJECT_DIR:-$PWD}"

if ! command -v docker >/dev/null 2>&1; then
	printf 'go-fmt-host: docker is required on PATH\n' >&2
	exit 127
fi

if [ ! -d "${project_dir}" ]; then
	printf 'go-fmt-host: project dir %s does not exist\n' "${project_dir}" >&2
	exit 1
fi

exec docker run --rm \
	-v "${project_dir}:/work" \
	-v "${cache_volume}:/cache" \
	-w /work \
	-e "HOST_PROJECT_PATH=${project_dir}" \
	-e GOCACHE=/cache/go-build \
	-e GOPATH=/cache/gopath \
	-e GOMODCACHE=/cache/gopath/pkg/mod \
	"${image}" "$@"
