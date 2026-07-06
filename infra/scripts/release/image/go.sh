#!/usr/bin/env bash

verify_go_image() {
	local tag="$1"
	local image
	local version_output
	local tmpdir
	local report_output
	local report_status

	image="$(release_image_ref "$tag")"
	version_output="$(release_image_run "$image" version)"
	assert_version_output "$image" "$version_output"

	tmpdir="$(create_release_image_tmpdir)"
	write_go_spacing_fixture "$tmpdir"

	set +e
	report_output="$(release_image_run_in_workdir "$tmpdir" "$image" check . 2>&1)"
	report_status=$?
	set -e

	printf '%s\n' "$report_output"

	assert_status 1 "$report_status" "expected check to exit 1 for ${image}"
	assert_output_contains "$report_output" "  sample.go"
	assert_output_contains "$report_output" "    [spacing] line 7: missing blank line after if statement"
	assert_output_contains "$report_output" "  Result: fail. 1 changed, 1 violation(s), 0 error(s)."
	assert_output_not_contains "$image" "$report_output" "~ sample.go:7 [spacing]" "legacy flat renderer detected"
}
