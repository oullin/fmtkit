#!/usr/bin/env bash
set -euo pipefail

# Smoke tests the shipped artifact rather than the code, which the Cargo tests
# (crates/cli/tests/smoke.rs) already cover: the release fmtkit binary and a
# version-stamped Go helper, in each layout a user can install them in.
#
#   archive    both binaries side by side, as in the cargo-dist tarball
#   homebrew   bin/fmtkit with the helper in share/fmtkit/ (formula pkgshare)
#
# Also checks that a missing helper and a helper from another release both
# fail with exit 3 instead of formatting with the wrong code.
#
# usage: test-binary-smoke.sh [archive.tar.xz]
#   With an archive (from `dist build`), tests its contents; otherwise builds
#   fmtkit and the helper for this machine. Requires bash, git, and go (for the
#   locally built helper and for go vet).

source "$(dirname "$0")/lib/env.sh"

tmp_root="$(mktemp -d)"

cleanup() {
	rm -rf "$tmp_root"
}

trap cleanup EXIT

dist="${tmp_root}/dist"

mkdir -p "$dist"

if (($# == 1)); then
	tar -xJf "$1" -C "$dist" --strip-components 1
else
	"${REPO_ROOT}/scripts/task.sh" build
	cp "${CARGO_TARGET_DIR}/release/fmtkit" "$(canonical_path "$GO_HELPER_DIR")/fmtkit-go-helper" "$dist/"
fi

for bin in fmtkit fmtkit-go-helper; do
	if [[ ! -x "${dist}/${bin}" ]]; then
		printf 'the artifact has no executable %s\n' "$bin" >&2
		exit 1
	fi
done

# Any helper override from the caller's environment would hide a broken lookup.
unset FMTKIT_GO_HELPER

export FMTKIT_CACHE_DIR="${tmp_root}/cache"

expected_version="fmtkit ${VERSION}"
expected_ts=$'const a = { s: \'hi\', x: 1 };\n\nexport default a;\n'
expected_go=$'package p\n\nfunc f() {\n\tdefer println("d")\n\n\treturn\n}\n'

fail() {
	printf '%s: %s\n' "$layout" "$1" >&2
	exit 1
}

# Writes a fresh fixture into $1 and runs format, check, and a second format
# with the fmtkit at $2.
exercise() {
	local fixture="$1" fmtkit="$2" status

	rm -rf "$fixture"
	mkdir -p "$fixture"
	git -C "$fixture" init --quiet .
	printf 'const  a = { x:1, s:"hi" }\nexport default a\n' > "${fixture}/app.ts"
	printf 'package p\n\nfunc f() {\n\tdefer println("d")\n\treturn\n}\n' > "${fixture}/app.go"
	printf 'module fixture\n\ngo 1.27.1\n' > "${fixture}/go.mod"

	[[ "$("$fmtkit" version)" == "$expected_version" ]] || fail "version printed $("$fmtkit" version), want ${expected_version}"

	(cd "$fixture" && "$fmtkit" format --all --quiet) || fail "format exited $?"

	diff <(printf '%s' "$expected_ts") "${fixture}/app.ts" || fail 'app.ts was not formatted as expected'
	diff <(printf '%s' "$expected_go") "${fixture}/app.go" || fail 'app.go was not formatted as expected'

	(cd "$fixture" && "$fmtkit" check --all --quiet --no-cache) || fail "check exited $? on a tree format had settled"

	status=0
	(cd "$fixture" && "$fmtkit" format --all --quiet --no-cache) || status=$?

	((status == 0)) || fail "a second format exited ${status}"
}

# Expects exit 3 from a format run whose helper is unusable.
expect_internal_failure() {
	local fixture="$1" fmtkit="$2" what="$3" status=0

	(cd "$fixture" && "$fmtkit" format --all --quiet --no-cache) 2> "${tmp_root}/stderr" || status=$?

	((status == 3)) || fail "${what}: exit ${status}, want 3"
	[[ -s "${tmp_root}/stderr" ]] || fail "${what}: no error message"
}

layout=archive
exercise "${tmp_root}/fixture-archive" "${dist}/fmtkit"

layout=homebrew
brew="${tmp_root}/Cellar/fmtkit/${VERSION}"
mkdir -p "${brew}/bin" "${brew}/share/fmtkit"
cp "${dist}/fmtkit" "${brew}/bin/"
cp "${dist}/fmtkit-go-helper" "${brew}/share/fmtkit/"
mkdir -p "${tmp_root}/prefix/bin"
ln -s "${brew}/bin/fmtkit" "${tmp_root}/prefix/bin/fmtkit"
exercise "${tmp_root}/fixture-homebrew" "${tmp_root}/prefix/bin/fmtkit"

layout='missing helper'
lonely="${tmp_root}/lonely"
mkdir -p "$lonely"
cp "${dist}/fmtkit" "$lonely/"
PATH=/usr/bin:/bin expect_internal_failure "${tmp_root}/fixture-archive" "${lonely}/fmtkit" 'no helper installed'

if command -v go > /dev/null; then
	layout='version mismatch'
	mismatched="${tmp_root}/mismatched"
	mkdir -p "$mismatched"
	cp "${dist}/fmtkit" "$mismatched/"
	CGO_ENABLED=0 go -C "${REPO_ROOT}/go/helper" build -trimpath -ldflags '-X main.version=0.0.0-smoke' -o "${mismatched}/fmtkit-go-helper" .
	expect_internal_failure "${tmp_root}/fixture-archive" "${mismatched}/fmtkit" 'helper from another release'
fi

printf 'binary smoke test passed\n'
