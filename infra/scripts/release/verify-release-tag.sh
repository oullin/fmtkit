#!/usr/bin/env bash
set -euo pipefail

if [ -z "${RELEASE_TAG:-}" ]; then
	printf 'RELEASE_TAG is required\n' >&2
	exit 1
fi

case "${RELEASE_TAG}" in
	v*) ;;
	*)
		printf 'release tag must start with v, got %s\n' "${RELEASE_TAG}" >&2
		exit 1
		;;
esac

git fetch --force origin "refs/tags/${RELEASE_TAG}:refs/tags/${RELEASE_TAG}"
git checkout --detach "refs/tags/${RELEASE_TAG}"

tag_sha="$(git rev-list -n1 "refs/tags/${RELEASE_TAG}")"
head_sha="$(git rev-parse HEAD)"

if [ "${tag_sha}" != "${head_sha}" ]; then
	printf 'refusing to publish release artifacts: checked out %s but tag %s points to %s\n' "${head_sha}" "${RELEASE_TAG}" "${tag_sha}" >&2
	exit 1
fi
