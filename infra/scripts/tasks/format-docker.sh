#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/docker-env.sh"

usage() {
	printf 'usage: %s <format|format-all|check> [paths...]\n' "${0##*/}" >&2
}

ensure_formatter_image() {
	local image="$FORMATTER_IMAGE"
	local dockerfile="$REPO_ROOT/$FORMATTER_DOCKERFILE"
	local policy="$FORMATTER_BUILD"
	local expected_version="fmtkit $VERSION"
	local expected_fingerprint="$FORMATTER_FINGERPRINT"
	local build_reason=''

	case "$policy" in
		auto)
			if ! docker image inspect "$image" >/dev/null 2>&1; then
				build_reason='missing'
			else
				image_version="$(docker run --rm "$image" version 2>/dev/null || true)"
				if [[ "$image_version" != "$expected_version" ]]; then
					build_reason='version changed'
				else
					image_fingerprint="$(docker image inspect "$image" --format '{{ index .Config.Labels "local.fmtkit.formatter-fingerprint" }}' 2>/dev/null || true)"
					if [[ "$image_fingerprint" != "$expected_fingerprint" ]]; then
						build_reason='support changed'
					fi
				fi
			fi
			;;
		always)
			build_reason='forced'
			;;
		never)
			if ! docker image inspect "$image" >/dev/null 2>&1; then
				printf 'formatter image is missing and FORMATTER_BUILD=never: %s\n' "$image" >&2
				printf 'Build it with `vp run image:full` or rerun with FORMATTER_BUILD=auto.\n' >&2
				exit 1
			fi
			;;
		*)
			printf 'invalid FORMATTER_BUILD value: %s\n' "$policy" >&2
			printf 'Expected one of: auto, always, never.\n' >&2
			exit 2
			;;
	esac

	if [[ -n "$build_reason" ]]; then
		printf 'Building formatter image %s (%s)...\n' "$image" "$build_reason" >&2
		docker build --build-arg "VERSION=$VERSION" --label "local.fmtkit.formatter-fingerprint=$expected_fingerprint" -f "$dockerfile" -t "$image" "$REPO_ROOT"
	fi
}

run_formatter() {
	local mode="$1"
	shift

	docker run --rm \
		-v "$FMTKIT_PROJECT_DIR:/work" \
		-v "$FMTKIT_CACHE_VOLUME:/cache" \
		-w /work \
		-e "HOST_PROJECT_PATH=$FMTKIT_PROJECT_DIR" \
		-e GOCACHE=/cache/go-build \
		-e GOPATH=/cache/gopath \
		-e GOMODCACHE=/cache/gopath/pkg/mod \
		"$FORMATTER_IMAGE" "$mode" "$@"
}

mode="${1:-}"
if [[ $# -gt 0 ]]; then
	shift
fi

if [[ "${1:-}" == "--" ]]; then
	shift
fi

case "$mode" in
	format)
		if [[ $# -eq 0 ]]; then
			set -- .
		fi
		ensure_formatter_image
		run_formatter format "$@"
		;;
	format-all)
		if [[ $# -ne 0 ]]; then
			usage
			exit 2
		fi
		ensure_formatter_image
		run_formatter format .
		;;
	check)
		if [[ $# -eq 0 ]]; then
			set -- .
		fi
		ensure_formatter_image
		run_formatter go check "$@"
		;;
	*)
		usage
		exit 2
		;;
esac
