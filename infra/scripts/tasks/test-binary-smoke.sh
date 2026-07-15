#!/usr/bin/env bash
set -euo pipefail

# Builds the self-contained fmtkit binary for the host platform and exercises
# the full pipeline against a scratch project: the bun-compiled sidecar, the
# oxc-parser/oxfmt/oxlint napi bindings, and the in-process Go formatter.
# Requires bash, git, go, node, npm, and bun.

repo_root="$(cd "$(dirname "$0")/../../.." && pwd)"

"${repo_root}/infra/scripts/release/stage-ts-assets.sh" host

tmp_root="$(mktemp -d)"

cleanup() {
	rm -rf "$tmp_root"
}

trap cleanup EXIT

bin="${tmp_root}/fmtkit"

(
	cd "$repo_root"

	go build -tags fmtkit_sidecar -o "$bin" ./packages/driver/cmd/fmtkit
)

fixture="${tmp_root}/fixture"

mkdir -p "$fixture"
cd "$fixture"

git init --quiet .

printf 'const  a = { x:1 }\nexport default a\n' > app.ts
printf 'package p\n\nfunc f() {\n\tdefer println("d")\n\treturn\n}\n' > app.go
printf 'module fixture\n\ngo 1.26.4\n' > go.mod

XDG_CACHE_HOME="${tmp_root}/cache" "$bin" version
XDG_CACHE_HOME="${tmp_root}/cache" "$bin" format .

expected_ts=$'const a = { x: 1 };\n\nexport default a;\n'
expected_go=$'package p\n\nfunc f() {\n\tdefer println("d")\n\n\treturn\n}\n'

if ! diff <(printf '%s' "$expected_ts") app.ts; then
	printf 'app.ts was not formatted as expected\n' >&2
	exit 1
fi

if ! diff <(printf '%s' "$expected_go") app.go; then
	printf 'app.go was not formatted as expected\n' >&2
	exit 1
fi

printf 'binary smoke test passed\n'
