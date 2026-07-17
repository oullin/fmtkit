#!/usr/bin/env bash
set -euo pipefail

# Builds the self-contained fmtkit binary for the host platform and exercises
# the full pipeline against a scratch project: the bun-compiled sidecar, the
# oxc-parser/oxfmt/oxlint napi bindings, and the in-process Go formatter.
# Requires bash, git, go, node, npm, and bun.

repo_root="$(cd "$(dirname "$0")/.." && pwd)"

"${repo_root}/packages/ts/infra/stage-ts-assets.sh" host

tmp_root="$(mktemp -d)"

cleanup() {
	rm -rf "$tmp_root"
}

trap cleanup EXIT

bin="${tmp_root}/fmtkit"

(
	cd "$repo_root"

	go -C packages/go build -tags fmtkit_sidecar -o "$bin" ./driver/cmd/fmtkit
)

fixture="${tmp_root}/fixture"

mkdir -p "$fixture"
cd "$fixture"

git init --quiet .

# The fixture carries no .oxfmtrc.* of its own, so oxfmt must pick up the
# bundled config. The double-quoted string is the probe: singleQuote there
# rewrites it, while a dropped config leaves oxfmt on its double-quote default.
printf 'const  a = { x:1, s:"hi" }\nexport default a\n' > app.ts
printf 'package p\n\nfunc f() {\n\tdefer println("d")\n\treturn\n}\n' > app.go
printf 'module fixture\n\ngo 1.26.4\n' > go.mod

# The Vue SFC is the embedded-formatter probe: its <template> and <style> blocks
# are formatted by oxfmt's external (prettier) formatter, the code path that a
# bun-compiled binary must run in-process (see stage-ts-assets.sh). If that path
# regresses, `format` hangs on this file instead of completing — which is exactly
# the failure this fixture guards against.
printf '<script setup lang="ts">\nconst  a = { x:1, s:"hi" }\n</script>\n\n<template>\n<div><p>{{ a.s }}</p></div>\n</template>\n\n<style scoped>\n.box{color:red;padding:0}\n</style>\n' > app.vue

XDG_CACHE_HOME="${tmp_root}/cache" "$bin" version
XDG_CACHE_HOME="${tmp_root}/cache" "$bin" format .

expected_ts=$'const a = { x: 1, s: \'hi\' };\n\nexport default a;\n'
expected_go=$'package p\n\nfunc f() {\n\tdefer println("d")\n\n\treturn\n}\n'
expected_vue=$'<script setup lang="ts">\nconst a = { x: 1, s: \'hi\' };\n</script>\n\n<template>\n\t<div>\n\t\t<p>{{ a.s }}</p>\n\t</div>\n</template>\n\n<style scoped>\n.box {\n\tcolor: red;\n\tpadding: 0;\n}\n</style>\n'

if ! diff <(printf '%s' "$expected_ts") app.ts; then
	printf 'app.ts was not formatted as expected\n' >&2
	exit 1
fi

if ! diff <(printf '%s' "$expected_go") app.go; then
	printf 'app.go was not formatted as expected\n' >&2
	exit 1
fi

if ! diff <(printf '%s' "$expected_vue") app.vue; then
	printf 'app.vue was not formatted as expected (embedded formatter regression?)\n' >&2
	exit 1
fi

printf 'binary smoke test passed\n'
