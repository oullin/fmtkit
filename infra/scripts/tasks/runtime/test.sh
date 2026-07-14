#!/usr/bin/env bash
# shellcheck source-path=SCRIPTDIR
set -euo pipefail

task_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
# shellcheck source=../env.sh
source "$task_dir/env.sh"
# shellcheck source=common.sh
source "$task_dir/runtime/common.sh"
# shellcheck source=platform.sh
source "$task_dir/runtime/platform.sh"
# shellcheck source=archive.sh
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
shellcheck_disable_directive='shellcheck disable'
shellcheck_disable_matches="$(find "$task_dir/runtime" -type f -name '*.sh' -exec grep -HnF -- "$shellcheck_disable_directive=" {} \;)"
if [[ -n "$shellcheck_disable_matches" ]]; then
	printf 'shellcheck disable directives are prohibited in runtime scripts:\n%s\n' "$shellcheck_disable_matches" >&2
	exit 1
fi
node_stub="$tmp/node"
cat >"$node_stub" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "${TEST_LIBC:-gnu}"
SH
chmod +x "$node_stub"
TEST_LIBC=gnu require_gnu_linux "$node_stub"
TEST_LIBC=musl assert_fails require_gnu_linux "$node_stub"
printf test >"$tmp/input"
assert_equals "$(portable_sha256 "$tmp/input")" "$(shasum -a 256 "$tmp/input" | awk '{print $1}')  input"

# GNU tar reports SIGPIPE under pipefail when grep -q stops at bin/node. Materialize
# the listing before checking it so native Linux runtime archives remain valid.
runtime_root="$tmp/runtime"
mkdir -p "$runtime_root/bin" "$runtime_root/go/bin" "$runtime_root/support/scripts" "$runtime_root/support/filler"
for member in node fmt-ts fmt-lint tsx oxfmt oxlint; do
	printf '#!/usr/bin/env sh\n' >"$runtime_root/bin/$member"
done
printf '#!/usr/bin/env sh\n' >"$runtime_root/go/bin/go"
printf 'export {}\n' >"$runtime_root/support/scripts/format-all.ts"
for number in $(seq 1 8192); do
	printf '%s\n' "$number" >"$runtime_root/support/filler/$number"
done
runtime_archive="$tmp/runtime-linux-amd64.tar.gz"
tar -C "$runtime_root" -czf "$runtime_archive" bin go support
validate_archive_members "$runtime_archive"
cp -R "$runtime_root" "$tmp/runtime-with-unexpected"
mkdir -p "$tmp/runtime-with-unexpected/unexpected"
printf 'unexpected\n' >"$tmp/runtime-with-unexpected/unexpected/member"
unexpected_archive="$tmp/runtime-with-unexpected.tar.gz"
tar -C "$tmp/runtime-with-unexpected" -czf "$unexpected_archive" bin go support unexpected
assert_fails validate_archive_members "$unexpected_archive"

verify="$task_dir/runtime/verify-binary.sh"
assert_fails "$verify"
assert_fails "$verify" --goos ' ' --goarch arm64 --binary /missing --checksum "$tmp/checksum"
assert_fails "$verify" --goos darwin --goarch --binary /missing --checksum "$tmp/checksum"
assert_fails "$verify" --goos darwin --goos darwin --goarch arm64 --binary /missing --checksum "$tmp/checksum"
assert_fails "$verify" --goos darwin --goarch arm64 --binary /missing --checksum "$tmp/checksum" extra

release="$task_dir/runtime/release.sh"
grep -Fq "fmtkit-\${goos}-\${goarch}" "$release"
grep -Fq './packages/driver/cmd/fmtkit' "$release"
if grep -Fq 'fmt-all-' "$release"; then
	printf 'contained release still exposes the old binary name\n' >&2
	exit 1
fi

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
