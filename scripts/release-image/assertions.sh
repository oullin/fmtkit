#!/usr/bin/env bash

release_image_fail() {
	printf '%s\n' "$1" >&2
	exit 1
}

assert_version_output() {
	local image="$1"
	local actual="$2"
	local expected="go-fmt ${NEW_TAG}"

	if [ "$actual" != "$expected" ]; then
		release_image_fail "unexpected version output for ${image}: ${actual}"
	fi
}

assert_status() {
	local expected="$1"
	local actual="$2"
	local message="$3"

	if [ "$actual" -ne "$expected" ]; then
		release_image_fail "${message}, got ${actual}"
	fi
}

assert_output_contains() {
	local output="$1"
	local expected="$2"

	grep -Fq "$expected" <<<"$output"
}

assert_output_not_contains() {
	local image="$1"
	local output="$2"
	local unexpected="$3"
	local message="$4"

	if grep -Fq "$unexpected" <<<"$output"; then
		release_image_fail "${message}: ${image}"
	fi
}

assert_file_contains() {
	local file="$1"
	local expected="$2"

	grep -Fq "$expected" "$file"
}
