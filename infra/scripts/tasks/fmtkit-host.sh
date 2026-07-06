#!/usr/bin/env bash
# fmtkit-host.sh — single-image, no-compose host wrapper for fmtkit.
#
# Every project on the host shares one selected image (FMTKIT_IMAGE, default
# ghcr.io/oullin/fmtkit:latest) and one named cache volume (FMTKIT_CACHE_VOLUME,
# default fmtkit-cache). Drop this script anywhere on $PATH (or symlink it as
# `fmtkit`) and invoke from any project root:
#
#     fmtkit-host.sh format .
#     fmtkit-host.sh format-all
#     fmtkit-host.sh go check ./pkg ./cmd
#     fmtkit-host.sh ts .
#
# Select a formatter flavor by exporting FMTKIT_IMAGE:
#
#     FMTKIT_IMAGE=ghcr.io/oullin/fmtkit:latest-full
#     FMTKIT_IMAGE=ghcr.io/oullin/fmtkit:latest-go
#     FMTKIT_IMAGE=ghcr.io/oullin/fmtkit:latest-node-ts
#
# Versioned flavor tags follow the same shape, for example v0.0.18-go,
# v0.0.18-node-ts, and v0.0.18-full.

set -euo pipefail

image="${FMTKIT_IMAGE:-ghcr.io/oullin/fmtkit:latest}"
cache_volume="${FMTKIT_CACHE_VOLUME:-fmtkit-cache}"
project_dir="${FMTKIT_PROJECT_DIR:-$PWD}"

if ! command -v docker >/dev/null 2>&1; then
	printf 'fmtkit-host: docker is required on PATH\n' >&2
	exit 127
fi

if [ ! -d "${project_dir}" ]; then
	printf 'fmtkit-host: project dir %s does not exist\n' "${project_dir}" >&2
	exit 1
fi

project_dir="$(cd "${project_dir}" && pwd)"

exec docker run --rm \
	-v "${project_dir}:/work" \
	-v "${cache_volume}:/cache" \
	-w /work \
	-e "HOST_PROJECT_PATH=${project_dir}" \
	-e GOCACHE=/cache/go-build \
	-e GOPATH=/cache/gopath \
	-e GOMODCACHE=/cache/gopath/pkg/mod \
	"${image}" "$@"
