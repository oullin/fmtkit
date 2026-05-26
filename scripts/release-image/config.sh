#!/usr/bin/env bash

if [ -z "${IMAGE_NAME:-}" ]; then
	printf 'IMAGE_NAME is required\n' >&2
	exit 1
fi

if [ -z "${NEW_TAG:-}" ]; then
	printf 'NEW_TAG is required\n' >&2
	exit 1
fi

release_image_ref() {
	local tag="$1"

	printf '%s:%s\n' "$IMAGE_NAME" "$tag"
}
