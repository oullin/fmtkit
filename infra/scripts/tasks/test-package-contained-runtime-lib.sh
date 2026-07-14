#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/package-contained-runtime-lib.sh"

assert_equals() {
	if [[ "$1" != "$2" ]]; then
		printf 'expected %q, got %q\n' "$2" "$1" >&2
		exit 1
	fi
}

assert_equals "$(go_arch_to_node_arch amd64)" x64
assert_equals "$(go_arch_to_node_arch arm64)" arm64
assert_equals "$(native_binding_suffix darwin arm64)" darwin-arm64
assert_equals "$(native_binding_suffix linux x64 gnu)" linux-x64-gnu
assert_equals "$(native_binding_suffix linux x64 musl)" linux-x64-musl

if go_arch_to_node_arch 386 >/dev/null 2>&1; then
	printf 'unsupported Go architecture was accepted\n' >&2
	exit 1
fi
