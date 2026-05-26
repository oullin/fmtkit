#!/usr/bin/env bash

verify_full_image() {
	local tag="$1"
	local image
	local version_output

	image="$(release_image_ref "$tag")"
	version_output="$(release_image_run "$image" version)"
	assert_version_output "$image" "$version_output"

	verify_go_image "$tag"

	version_output="$(release_image_run "$image" go version)"
	if [ "$version_output" != "go-fmt ${NEW_TAG}" ]; then
		release_image_fail "full image did not forward go version correctly: ${image}"
	fi
}
