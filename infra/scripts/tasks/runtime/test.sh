#!/usr/bin/env bash
set -euo pipefail

task_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
source "$task_dir/env.sh"
source "$task_dir/runtime/common.sh"
source "$task_dir/runtime/platform.sh"
source "$task_dir/runtime/archive.sh"

assert_equals() { [[ "$1" == "$2" ]] || { printf 'expected %q, got %q\n' "$2" "$1" >&2; exit 1; }; }
assert_fails() {
	if "$@" >/dev/null 2>&1; then
		printf 'expected failure: %s\n' "$*" >&2
		exit 1
	fi
}

assert_equals "$(trim '  value  ')" value
assert_equals "$(go_arch_to_node_arch amd64)" x64
assert_equals "$(go_arch_to_node_arch arm64)" arm64
assert_fails go_arch_to_node_arch 386
validate_runtime_platform darwin arm64
validate_runtime_platform linux amd64
validate_runtime_platform linux arm64
assert_fails validate_runtime_platform darwin amd64

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
node_stub="$tmp/node"
cat >"$node_stub" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "${TEST_LIBC:-gnu}"
SH
chmod +x "$node_stub"
TEST_LIBC=gnu require_gnu_linux "$node_stub"
assert_fails env TEST_LIBC=musl bash -c 'source "$1"; source "$2"; require_gnu_linux "$3"' bash "$task_dir/runtime/common.sh" "$task_dir/runtime/platform.sh" "$node_stub"
printf test >"$tmp/input"
assert_equals "$(portable_sha256 "$tmp/input")" "$(shasum -a 256 "$tmp/input" | awk '{print $1}')  input"

verify="$task_dir/runtime/verify-binary.sh"
assert_fails "$verify"
assert_fails "$verify" --goos ' ' --goarch arm64 --binary /missing --checksum "$tmp/checksum"
assert_fails "$verify" --goos darwin --goarch --binary /missing --checksum "$tmp/checksum"
assert_fails "$verify" --goos darwin --goos darwin --goarch arm64 --binary /missing --checksum "$tmp/checksum"
assert_fails "$verify" --goos darwin --goarch arm64 --binary /missing --checksum "$tmp/checksum" extra

manifest_verify="$task_dir/runtime/verify-manifest-platform.sh"
manifest="$tmp/runtime-linux-amd64.tar.gz.manifest.json"
cat >"$manifest" <<'JSON'
{"archive_sha256":"0000000000000000000000000000000000000000000000000000000000000000","goos":"linux","goarch":"amd64","tree_sha256":"0000000000000000000000000000000000000000000000000000000000000000","required":["bin/node"]}
JSON
"$manifest_verify" --goos linux --goarch amd64 --manifest "$manifest"
assert_fails "$manifest_verify" --goos linux --goarch arm64 --manifest "$manifest"
assert_fails "$manifest_verify" --goos linux --goarch amd64 --manifest "$(dirname "$manifest")/runtime-linux-arm64.tar.gz.manifest.json"
printf '{"goos":" ","goarch":"amd64"}\n' >"$manifest"
assert_fails "$manifest_verify" --goos linux --goarch amd64 --manifest "$manifest"
