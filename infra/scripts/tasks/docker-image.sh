#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/docker-env.sh"

usage() {
	printf 'usage: %s <go|node-ts|full|clean>\n' "${0##*/}" >&2
}

mode="${1:-}"

case "$mode" in
	go)
		docker build --build-arg "VERSION=$VERSION" -f "$REPO_ROOT/infra/docker/Dockerfile.go" -t "$GO_IMAGE" "$REPO_ROOT"
		;;
	node-ts)
		docker build --label "local.fmtkit.formatter-fingerprint=$FORMATTER_FINGERPRINT" -f "$REPO_ROOT/infra/docker/Dockerfile.node-ts" -t "$NODE_TS_IMAGE" "$REPO_ROOT"
		;;
	full)
		docker build --build-arg "VERSION=$VERSION" --label "local.fmtkit.formatter-fingerprint=$FORMATTER_FINGERPRINT" -f "$REPO_ROOT/infra/docker/Dockerfile.full" -t "$FULL_IMAGE" "$REPO_ROOT"
		;;
	clean)
		containers="$(docker ps -aq --filter "ancestor=$GO_IMAGE" --filter "ancestor=$NODE_TS_IMAGE" --filter "ancestor=$FULL_IMAGE")"
		if [[ -n "$containers" ]]; then
			printf '%s\n' "$containers" | xargs docker rm -f
		fi

		docker rmi -f "$GO_IMAGE" "$NODE_TS_IMAGE" "$FULL_IMAGE" 2>/dev/null || true

		images="$(docker images -q --filter label=local.fmtkit.formatter-fingerprint | sort -u)"
		if [[ -n "$images" ]]; then
			printf '%s\n' "$images" | xargs docker rmi -f
		fi

		docker volume rm "$FMTKIT_CACHE_VOLUME" 2>/dev/null || true
		docker image prune -f --filter label=local.fmtkit.formatter-fingerprint
		;;
	*)
		usage
		exit 2
		;;
esac
