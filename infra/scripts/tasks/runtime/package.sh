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
# shellcheck source=dependencies.sh
source "$task_dir/runtime/dependencies.sh"
# shellcheck source=archive.sh
source "$task_dir/runtime/archive.sh"

require_no_extra_args "$@"
goos="$(require_nonblank RUNTIME_GOOS "${RUNTIME_GOOS:-$(go -C "$GO_WORKDIR" env GOOS)}")"
goarch="$(require_nonblank RUNTIME_GOARCH "${RUNTIME_GOARCH:-$(go -C "$GO_WORKDIR" env GOARCH)}")"
NODE_BIN="$(require_nonblank NODE_BIN "${NODE_BIN:-$(command -v node)}")"
go_root="$(require_nonblank GO_ROOT "${GO_ROOT:-$(go -C "$GO_WORKDIR" env GOROOT)}")"

validate_runtime_platform "$goos" "$goarch"
[[ -x "$NODE_BIN" ]] || { printf 'Node.js executable is required to package the contained runtime\n' >&2; exit 1; }
validate_native_platform "$goos" "$goarch" "$NODE_BIN"

stage_root="$(mktemp -d)"
stage="$stage_root/runtime"
cleanup() { rm -rf "$stage_root"; }
trap cleanup EXIT

mkdir -p "$stage/bin" "$stage/lib" "$stage/support/cmd" "$stage/support/scripts"
CI=true vp install --frozen-lockfile
cp "$NODE_BIN" "$stage/bin/node"
cp -R "$go_root" "$stage/go"

cat >"$stage/bin/go" <<'SH'
#!/usr/bin/env sh
root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)"
export GOROOT="$root/go"
exec "$root/go/bin/go" "$@"
SH

node_lib_dir="$(cd "$(dirname "$NODE_BIN")/../lib" 2>/dev/null && pwd -P || true)"
if [[ -n "$node_lib_dir" ]]; then
	shopt -s nullglob
	node_libs=("$node_lib_dir"/libnode*.dylib "$node_lib_dir"/libnode*.so*)
	shopt -u nullglob
	((${#node_libs[@]} == 0)) || cp "${node_libs[@]}" "$stage/lib/"
fi

cp "$REPO_ROOT"/infra/bin/{fmtkit-ts,fmtkit-ts-files,fmtkit-lint,fmtkit-ts-lint} "$stage/support/cmd/"
cp "$REPO_ROOT"/packages/devx/scripts/*.ts "$stage/support/scripts/"
cp "$REPO_ROOT/packages/devx/scripts/package.json" "$stage/support/scripts/package.json"
cp "$REPO_ROOT"/.ox{fmt,lint}rc.json "$stage/support/"
pnpm --filter devx deploy --legacy --dev "$stage/devx-closure"
rm -f "$stage/devx-closure/node_modules/.pnpm/node_modules/devx"
cp -RL "$stage/devx-closure/node_modules" "$stage/support/node_modules"
rm -rf "$stage/devx-closure"

node_arch="$(go_arch_to_node_arch "$goarch")"
binding_suffix="$goos-$node_arch"
if [[ "$goos" == linux ]]; then binding_suffix+="-gnu"; fi
copy_locked_package esbuild "$stage/support/node_modules/esbuild" tsx
copy_locked_package "@esbuild/$goos-$node_arch" "$stage/support/node_modules/@esbuild/$goos-$node_arch" esbuild
copy_locked_package "@oxc-parser/binding-$binding_suffix" "$stage/support/node_modules/@oxc-parser/binding-$binding_suffix" oxc-parser
copy_locked_package tinypool "$stage/support/node_modules/tinypool" oxfmt
copy_locked_package "@oxfmt/binding-$binding_suffix" "$stage/support/node_modules/@oxfmt/binding-$binding_suffix" oxfmt
copy_locked_package "@oxlint/binding-$binding_suffix" "$stage/support/node_modules/@oxlint/binding-$binding_suffix" oxlint

cat >"$stage/bin/tsx" <<'SH'
#!/usr/bin/env sh
root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)"
exec "$root/bin/node" "$root/support/node_modules/tsx/dist/cli.mjs" "$@"
SH

for tool in oxfmt oxlint; do
	cat >"$stage/bin/$tool" <<SH
#!/usr/bin/env sh
root="\$(CDPATH= cd -- "\$(dirname -- "\$0")/.." && pwd -P)"
exec "\$root/bin/node" "\$root/support/node_modules/$tool/bin/$tool" "\$@"
SH
done

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
assets_dir="$(runtime_assets_dir)"
mkdir -p "$assets_dir"
archive="$(runtime_archive_path "$goos" "$goarch")"
COPYFILE_DISABLE=1 tar -h -C "$stage" -czf "$archive" bin go lib support
go -C "$GO_WORKDIR" run ./packages/driver/cmd/runtime-manifest \
	--root "$stage" \
	--archive "$archive" \
	--output "$archive.manifest.json" \
	--goos "$goos" \
	--goarch "$goarch" \
	--required 'bin/node,bin/fmt-ts,bin/fmt-lint,bin/tsx,bin/oxfmt,bin/oxlint,go/bin/go,support/scripts/format-all.ts'

printf 'Wrote %s and %s.manifest.json\n' "$archive" "$archive"
