#!/usr/bin/env bash

verify_node_ts_image() {
	local tag="$1"
	local image
	local tmpdir

	image="$(release_image_ref "$tag")"
	tmpdir="$(create_release_image_tmpdir)"
	write_node_ts_fixture "$tmpdir"

	release_image_run_in_workdir "$tmpdir" "$image" .
	assert_file_contains "${tmpdir}/sample.ts" 'const value = { name: "demo" };'

	if release_image_run "$image" go version >/dev/null 2>&1; then
		release_image_fail "node-ts image unexpectedly accepts Go CLI commands: ${image}"
	fi
}
