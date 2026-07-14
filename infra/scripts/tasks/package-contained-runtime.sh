#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/env.sh"

goos="${RUNTIME_GOOS:-$(go -C "$GO_WORKDIR" env GOOS)}"
goarch="${RUNTIME_GOARCH:-$(go -C "$GO_WORKDIR" env GOARCH)}"
node_bin="${NODE_BIN:-$(command -v node)}"
npm_bin="${NPM_BIN:-$(command -v npm)}"
go_root="${GO_ROOT:-$(go -C "$GO_WORKDIR" env GOROOT)}"
assets_dir="$REPO_ROOT/packages/driver/internal/full/assets"
stage_root="$(mktemp -d)"
stage="$stage_root/runtime"

cleanup() {
	rm -rf "$stage_root"
}

trap cleanup EXIT

mkdir -p "$stage/bin" "$stage/lib" "$stage/support/cmd" "$stage/support/scripts"

cp "$node_bin" "$stage/bin/node"
cp -R "$go_root" "$stage/go"

cat >"$stage/bin/go" <<'SH'
#!/usr/bin/env sh
root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)"
export GOROOT="$root/go"
exec "$root/go/bin/go" "$@"
SH

node_lib_dir="$(cd "$(dirname "$node_bin")/../lib" 2>/dev/null && pwd -P || true)"

if [[ -n "$node_lib_dir" ]]; then
	shopt -s nullglob
	node_libs=("$node_lib_dir"/libnode*.dylib "$node_lib_dir"/libnode*.so*)
	shopt -u nullglob

	if [[ ${#node_libs[@]} -gt 0 ]]; then
		cp "${node_libs[@]}" "$stage/lib/"
	fi
fi

cp "$REPO_ROOT/infra/bin/fmtkit-ts" "$stage/support/cmd/fmtkit-ts"
cp "$REPO_ROOT/infra/bin/fmtkit-ts-files" "$stage/support/cmd/fmtkit-ts-files"
cp "$REPO_ROOT/infra/bin/fmtkit-lint" "$stage/support/cmd/fmtkit-lint"
cp "$REPO_ROOT/infra/bin/fmtkit-ts-lint" "$stage/support/cmd/fmtkit-ts-lint"
cp "$REPO_ROOT"/packages/devx/scripts/*.ts "$stage/support/scripts/"
cp "$REPO_ROOT/packages/devx/scripts/package.json" "$stage/support/scripts/package.json"
cp "$REPO_ROOT/.oxfmtrc.json" "$stage/support/.oxfmtrc.json"
cp "$REPO_ROOT/.oxlintrc.json" "$stage/support/.oxlintrc.json"

deps="$("$node_bin" -e '
const p = require(process.argv[1]);
const deps = ["oxc-parser", "oxfmt", "oxlint", "tsx"];
console.log(deps.map((name) => `${name}@${p.devDependencies[name]}`).join(" "));
' "$REPO_ROOT/packages/devx/package.json")"

"$npm_bin" install --prefix "$stage/support" --no-save $deps >/dev/null

cat >"$stage/bin/tsx" <<'SH'
#!/usr/bin/env sh
root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)"
exec "$root/bin/node" "$root/support/node_modules/tsx/dist/cli.mjs" "$@"
SH

cat >"$stage/bin/oxfmt" <<'SH'
#!/usr/bin/env sh
root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)"
exec "$root/bin/node" "$root/support/node_modules/oxfmt/bin/oxfmt" "$@"
SH

cat >"$stage/bin/oxlint" <<'SH'
#!/usr/bin/env sh
root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)"
exec "$root/bin/node" "$root/support/node_modules/oxlint/bin/oxlint" "$@"
SH

cat >"$stage/bin/fmt-ts" <<'SH'
#!/usr/bin/env sh
root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)"
export FMTKIT_SUPPORT_DIR="$root/support"
export FMTKIT_SOURCES_BIN="$root/bin/fmt-sources"
export FMTKIT_FORMAT_ALL_SCRIPT="$root/support/scripts/format-all.ts"
export TSX_BIN="$root/bin/tsx"
export OXFMT_BIN="$root/bin/oxfmt"
exec "$root/support/cmd/fmtkit-ts" "$@"
SH

cat >"$stage/bin/fmt-lint" <<'SH'
#!/usr/bin/env sh
root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)"
export FMTKIT_SUPPORT_DIR="$root/support"
export FMTKIT_SOURCES_BIN="$root/bin/fmt-sources"
export OXLINT_BIN="$root/bin/oxlint"
exec "$root/support/cmd/fmtkit-lint" "$@"
SH

chmod +x "$stage/bin/"* "$stage/support/cmd/"*
mkdir -p "$assets_dir"

archive="$assets_dir/runtime-${goos}-${goarch}.tar.gz"
COPYFILE_DISABLE=1 tar -C "$stage" -czf "$archive" bin go lib support

printf 'Wrote %s\n' "$archive"
