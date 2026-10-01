#!/usr/bin/env bash
set -euo pipefail

# Builds the self-contained fmtkit binary for the host platform and exercises
# the full pipeline against a scratch project: the bun-compiled sidecar, the
# oxc-parser/oxfmt/oxlint napi bindings, and the in-process Go formatter.
# Requires bash, git, go, node, npm, and bun.

repo_root="$(cd "$(dirname "$0")/.." && pwd)"

"${repo_root}/packages/ts/toolchain/stage-ts-assets.sh" host

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
printf 'module fixture\n\ngo 1.27.1\n' > go.mod

# The Vue SFC is the embedded-formatter probe: its <template> and <style> blocks
# are formatted by oxfmt's external (prettier) formatter, the code path that a
# bun-compiled binary must run in-process (see stage-ts-assets.sh). If that path
# regresses, `format` hangs on this file instead of completing — which is exactly
# the failure this fixture guards against.
printf '<script setup lang="ts">\nconst  a = { x:1, s:"hi" }\n</script>\n\n<template>\n<div><p>{{ a.s }}</p></div>\n</template>\n\n<style scoped>\n.box{color:red;padding:0}\n</style>\n' > app.vue

XDG_CACHE_HOME="${tmp_root}/cache" "$bin" version
XDG_CACHE_HOME="${tmp_root}/cache" "$bin" format .

expected_ts=$'const a = { s: \'hi\', x: 1 };\n\nexport default a;\n'
expected_go=$'package p\n\nfunc f() {\n\tdefer println("d")\n\n\treturn\n}\n'
expected_vue=$'<script setup lang="ts">\nconst a = { s: \'hi\', x: 1 };\n</script>\n\n<template>\n\t<div>\n\t\t<p>{{ a.s }}</p>\n\t</div>\n</template>\n\n<style scoped>\n.box {\n\tcolor: red;\n\tpadding: 0;\n}\n</style>\n'

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

# Formatting must reach a fixed point. A pass that keeps rewriting already-formatted
# source turns `format --check` into a permanent failure and makes every run a diff,
# and the first pass alone cannot reveal it: the output above is correct either way.
# `--fix` runs inside this pipeline too, so a fixer that oscillates lands here first.
before_second="${tmp_root}/before-second"
after_second="${tmp_root}/after-second"

find . -type f -not -path './.git/*' -exec shasum {} + | sort > "$before_second"

XDG_CACHE_HOME="${tmp_root}/cache" "$bin" format .

find . -type f -not -path './.git/*' -exec shasum {} + | sort > "$after_second"

if ! diff "$before_second" "$after_second"; then
	printf 'format is not idempotent: a second pass over formatted source changed it\n' >&2
	exit 1
fi

# `check` is the same pipeline reporting instead of writing, so it must agree that
# the tree is settled. Disagreement means check and format apply different rules.
if ! XDG_CACHE_HOME="${tmp_root}/cache" "$bin" check .; then
	printf 'check reported changes on a tree format had just settled\n' >&2
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

printf '// Project exceptions overlay the bundled policy; duplicate plugin aliases keep the embedded copy.\n{\n\t"jsPlugins": [{ "name": "@nkzw", "specifier": "./missing-because-bundled-wins.mjs" }],\n\t"rules": {\n\t\t"require-await": "off"\n\t}\n}\n' > .oxlintrc.jsonc

printf 'export type Foo = { a: number };\n' > types.ts
printf "import { Foo } from './types';\n\nexport const value: Foo = { a: 1 };\n" > uses.ts
printf 'export async function localException(): Promise<number> {\n\treturn 1;\n}\n' > local-exception.ts

# One probe per plugin the config names. oxlint's `plugins` field *overwrites*
# the base set rather than extending it, so dropping a name here silently
# switches its rules off — which is how the oxc plugin went missing once already.
printf 'export function erasing(y: number): number {\n\treturn y * 0;\n}\n' > oxc.ts
printf "import path from 'path';\n\nexport const p = path.sep;\n" > unicorn.ts
printf 'const a = 1;\nconst b = 2;\n\nexport { a as dup };\nexport { b as dup };\n' > importdup.ts

mkdir -p generated
printf '{"rules":{"typescript/no-explicit-any":"off"}}\n' > generated/.oxlintrc.json
printf 'export const generatedValue: any = 1;\n' > generated/client.ts

lint_log="${tmp_root}/lint.log"

if XDG_CACHE_HOME="${tmp_root}/cache" "$bin" lint . > "$lint_log" 2>&1; then
	printf 'lint exited 0 on files that violate the bundled config\n' >&2
	cat "$lint_log" >&2
	exit 1
fi

for rule in 'typescript(consistent-type-imports)' 'oxc(erasing-op)' 'unicorn(prefer-node-protocol)' 'import(export)'; do
	if ! grep -qF "$rule" "$lint_log"; then
		printf 'the bundled oxlint config did not report %s\n' "$rule" >&2
		cat "$lint_log" >&2
		exit 1
	fi
done

# The shipped binary must load every JS plugin without Node or a project
# node_modules. Restrict PATH to git, the only external executable lint needs.
printf 'const value = new Date();\nexport const bad = value instanceof Date;\nexport const sorted = { zebra: 1, alpha: 2 };\ntest.only("focused", () => {});\n' > bundled-plugins.ts
printf 'export const Button = () => <button>Save</button>;\n' > react.tsx
node_free_path="${tmp_root}/node-free-path"
mkdir -p "$node_free_path"
ln -s "$(command -v git)" "${node_free_path}/git"
plugin_log="${tmp_root}/plugins.log"

if PATH="$node_free_path" XDG_CACHE_HOME="${tmp_root}/cache" "$bin" lint bundled-plugins.ts react.tsx > "$plugin_log" 2>&1; then
	printf 'lint exited 0 on bundled plugin and React rule violations\n' >&2
	cat "$plugin_log" >&2
	exit 1
fi

for rule in '@nkzw(no-instanceof)' 'no-only-tests(no-only-tests)' 'perfectionist(sort-objects)' 'react(button-has-type)'; do
	if ! grep -qF "$rule" "$plugin_log"; then
		printf 'bundled policy did not report %s without Node\n' "$rule" >&2
		cat "$plugin_log" >&2
		exit 1
	fi
done

mkdir -p missing-plugin
printf '{"jsPlugins":[{"name":"missing","specifier":"./not-installed.mjs"}]}\n' > missing-plugin/.oxlintrc.json
printf 'export const valid = 1;\n' > missing-plugin/app.ts

if XDG_CACHE_HOME="${tmp_root}/cache" "$bin" lint missing-plugin/app.ts > "${tmp_root}/missing-plugin.log" 2>&1; then
	printf 'lint silently accepted a JS plugin that could not be loaded\n' >&2
	exit 1
fi

if ! grep -q 'Failed to load JS plugin' "${tmp_root}/missing-plugin.log"; then
	printf 'missing JS plugin did not produce a visible load error\n' >&2
	cat "${tmp_root}/missing-plugin.log" >&2
	exit 1
fi

for rule in 'require-await' 'typescript(no-explicit-any)'; do
	if grep -qF "$rule" "$lint_log"; then
		printf 'the project oxlint overlay did not override %s\n' "$rule" >&2
		cat "$lint_log" >&2
		exit 1
	fi
done

if find . -name '.fmtkit-oxlint-*' -print -quit | grep -q .; then
	printf 'lint left a composed oxlint config in the project\n' >&2
	find . -name '.fmtkit-oxlint-*' -print >&2
	exit 1
fi

# An imported project preset uses the project's Node-based Oxlint. The bundled
# default still applies, and duplicate JS plugin names are resolved in favour
# of the later project config.
import_fixture="${tmp_root}/import-fixture"
mkdir -p "$import_fixture/nested"
cd "$import_fixture"
git init --quiet .
printf '{"private":true,"name":"fmtkit-import-fixture","version":"0.0.0","type":"module"}\n' > package.json
npm install --no-save --no-audit --no-fund --prefix . oxlint@1.86.0 @nkzw/oxlint-config@2.0.1 >/dev/null
printf "import nkzw from '@nkzw/oxlint-config';\nimport { defineConfig } from 'oxlint';\nexport default defineConfig({ extends: [nkzw] });\n" > oxlint.config.ts
printf 'const value = new Date();\nexport const bad = value instanceof Date;\n' > imported.ts
printf '{"rules":{"@nkzw/no-instanceof":"off"}}\n' > nested/.oxlintrc.json
cp imported.ts nested/imported.ts

if XDG_CACHE_HOME="${tmp_root}/cache" "$bin" lint imported.ts > "${tmp_root}/imported.log" 2>&1; then
	printf 'imported preset did not report its custom rule\n' >&2
	exit 1
fi

if ! grep -qF '@nkzw(no-instanceof)' "${tmp_root}/imported.log"; then
	printf 'imported preset failed to load or lost the bundled default\n' >&2
	cat "${tmp_root}/imported.log" >&2
	exit 1
fi

XDG_CACHE_HOME="${tmp_root}/cache" "$bin" lint nested/imported.ts

if find . -name '.fmtkit-oxlint-*' -print -quit | grep -q .; then
	printf 'imported lint left a composed config in the project\n' >&2
	exit 1
fi

# Every JavaScript dialect gets the same lint-fix, oxfmt, structural-pass, and
# syntax-validation schedule as TypeScript. Lint itself never writes.
js_fixture="${tmp_root}/js-fixture"
mkdir -p "$js_fixture"
cd "$js_fixture"
git init --quiet .
printf 'export const result={zebra:1,alpha:2};\n' > app.js
printf 'export const Widget=()=> <button type="button">Go</button>;\n' > Widget.jsx
printf 'export const ready= true;\n' > module.mjs
printf 'module.exports={zebra:1,alpha:2};\n' > legacy.cjs
shasum app.js Widget.jsx module.mjs legacy.cjs > "${tmp_root}/js-before-lint"

if XDG_CACHE_HOME="${tmp_root}/cache" "$bin" lint . > "${tmp_root}/js-lint.log" 2>&1; then
	printf 'JavaScript lint missed unsorted objects\n' >&2
	exit 1
fi

shasum app.js Widget.jsx module.mjs legacy.cjs > "${tmp_root}/js-after-lint"
diff "${tmp_root}/js-before-lint" "${tmp_root}/js-after-lint"
XDG_CACHE_HOME="${tmp_root}/cache" "$bin" format-all --ts > "${tmp_root}/js-format.log"
XDG_CACHE_HOME="${tmp_root}/cache" "$bin" lint .
shasum app.js Widget.jsx module.mjs legacy.cjs > "${tmp_root}/js-first-format"
XDG_CACHE_HOME="${tmp_root}/cache" "$bin" format-all --ts > "${tmp_root}/js-second-format.log"
shasum app.js Widget.jsx module.mjs legacy.cjs > "${tmp_root}/js-second-format"
diff "${tmp_root}/js-first-format" "${tmp_root}/js-second-format"

# `no-fallthrough` ships in the bundled config, and oxlint reports an empty
# `case` label as a fallthrough once a blank line splits it from the label
# below — the blank-line pass used to insert exactly that. Format first, then
# lint what came out: the two halves of the binary have to agree on the result.
fallthrough_fixture="${tmp_root}/fallthrough-fixture"

mkdir -p "$fallthrough_fixture"
cd "$fallthrough_fixture"

git init --quiet .

printf "export function classify(value: string): number {\n\tswitch (value) {\n\t\tcase 'a':\n\t\tcase 'b':\n\t\t\treturn 1;\n\t\tdefault:\n\t\t\treturn 0;\n\t}\n}\n" > switch.ts

XDG_CACHE_HOME="${tmp_root}/cache" "$bin" format .

expected_switch=$'export function classify(value: string): number {\n\tswitch (value) {\n\t\tcase \'a\':\n\t\tcase \'b\':\n\t\t\treturn 1;\n\n\t\tdefault:\n\t\t\treturn 0;\n\t}\n}\n'

if ! diff <(printf '%s' "$expected_switch") switch.ts; then
	printf 'the formatter split a grouped case label group\n' >&2
	exit 1
fi

fallthrough_log="${tmp_root}/fallthrough.log"

if ! XDG_CACHE_HOME="${tmp_root}/cache" "$bin" lint . > "$fallthrough_log" 2>&1; then
	printf "lint rejected the formatter's own output for grouped case labels\n" >&2
	cat "$fallthrough_log" >&2
	exit 1
fi

printf 'binary smoke test passed\n'
