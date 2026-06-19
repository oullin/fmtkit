#!/usr/bin/env bash

source "$(dirname "${BASH_SOURCE[0]}")/env.sh"

export VERSION="${VERSION:-$(git -C "$REPO_ROOT" describe --tags --always --dirty 2>/dev/null || echo dev)}"
export FORMATTER_IMAGE="${FORMATTER_IMAGE:-go-fmt-full:local}"
export FORMATTER_DOCKERFILE="${FORMATTER_DOCKERFILE:-docker/Dockerfile.full}"
export FORMATTER_BUILD="${FORMATTER_BUILD:-auto}"
export GO_IMAGE="${GO_IMAGE:-go-fmt-go:local}"
export NODE_TS_IMAGE="${NODE_TS_IMAGE:-go-fmt-node-ts:local}"
export FULL_IMAGE="${FULL_IMAGE:-go-fmt-full:local}"
export GO_FMT_CACHE_VOLUME="${GO_FMT_CACHE_VOLUME:-go-fmt-cache}"
export GO_FMT_PROJECT_DIR="${GO_FMT_PROJECT_DIR:-$REPO_ROOT}"

formatter_fingerprint() {
	{
		printf '%s\n' \
			"$FORMATTER_DOCKERFILE" \
			package.json \
			pnpm-lock.yaml \
			cmd/fmt-ts \
			cmd/fmt-ts-files.sh \
			cmd/fmt-lint \
			cmd/fmt-ts-lint.sh \
			cmd/fmt-all \
			.oxlintrc.json \
			packages/devx/package.json \
			packages/devx/scripts/package.json
		(cd "$REPO_ROOT" && find packages/devx/scripts -maxdepth 1 -type f -name '*.ts')
	} | sort | while IFS= read -r file; do
		[[ -f "$REPO_ROOT/$file" ]] && shasum -a 256 "$REPO_ROOT/$file"
	done | shasum -a 256 | awk '{print $1}'
}

export FORMATTER_FINGERPRINT="${FORMATTER_FINGERPRINT:-$(formatter_fingerprint)}"
