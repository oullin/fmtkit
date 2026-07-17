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

# The lint step has to enforce the bundled .oxlintrc, typescript rules included.
# This fails open, which is why it is worth a fixture: without the config oxlint
# still runs and still exits 0, it just stops reporting anything the config
# turned on. consistent-type-imports is the probe because it fires only when the
# bundled config reaches oxlint — it is off in oxlint's defaults. Kept in its own
# fixture so the exit code belongs to lint alone.
lint_fixture="${tmp_root}/lint-fixture"

mkdir -p "$lint_fixture"
cd "$lint_fixture"

git init --quiet .

printf 'export type Foo = { a: number };\n' > types.ts
printf "import { Foo } from './types';\n\nexport const value: Foo = { a: 1 };\n" > uses.ts

# One probe per plugin the config names. oxlint's `plugins` field *overwrites*
# the base set rather than extending it, so dropping a name here silently
# switches its rules off — which is how the oxc plugin went missing once already.
printf 'export function erasing(y: number): number {\n\treturn y * 0;\n}\n' > oxc.ts

lint_log="${tmp_root}/lint.log"

if XDG_CACHE_HOME="${tmp_root}/cache" "$bin" lint . > "$lint_log" 2>&1; then
	printf 'lint exited 0 on files that violate the bundled config\n' >&2
	cat "$lint_log" >&2
	exit 1
fi

for rule in 'typescript(consistent-type-imports)' 'oxc(erasing-op)'; do
	if ! grep -qF "$rule" "$lint_log"; then
		printf 'the bundled oxlint config did not report %s\n' "$rule" >&2
		cat "$lint_log" >&2
		exit 1
	fi
done

printf 'binary smoke test passed\n'
