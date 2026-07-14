#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/env.sh"
source "$(dirname "$0")/package-contained-runtime-lib.sh"

goos="${RUNTIME_GOOS:-$(go -C "$GO_WORKDIR" env GOOS)}"
goarch="${RUNTIME_GOARCH:-$(go -C "$GO_WORKDIR" env GOARCH)}"
node_bin="${NODE_BIN:-$(command -v node)}"
go_root="${GO_ROOT:-$(go -C "$GO_WORKDIR" env GOROOT)}"
assets_dir="$REPO_ROOT/packages/driver/internal/full/assets"
stage_root="$(mktemp -d)"
stage="$stage_root/runtime"

if [[ -z "$node_bin" || ! -x "$node_bin" ]]; then
	printf 'Node.js executable is required to package the contained runtime\n' >&2
	exit 1
fi

cleanup() {
	rm -rf "$stage_root"
}

trap cleanup EXIT

mkdir -p "$stage/bin" "$stage/lib" "$stage/support/cmd" "$stage/support/scripts"

native_goos="$(go -C "$GO_WORKDIR" env GOOS)"
native_goarch="$(go -C "$GO_WORKDIR" env GOARCH)"
native_node_platform="$("$node_bin" -p 'process.platform')"
native_node_arch="$("$node_bin" -p 'process.arch')"
native_go_node_arch="$(go_arch_to_node_arch "$native_goarch")" || {
	printf 'unsupported Go architecture: %s\n' "$native_goarch" >&2
	exit 1
}

if [[ "$goos" != "$native_goos" || "$goarch" != "$native_goarch" || "$goos" != "$native_node_platform" || "$native_go_node_arch" != "$native_node_arch" ]]; then
	printf 'contained runtime packaging must run natively; requested %s/%s, Go is %s/%s, Node is %s/%s\n' "$goos" "$goarch" "$native_goos" "$native_goarch" "$native_node_platform" "$native_node_arch" >&2
	exit 1
fi

CI=true vp install --frozen-lockfile

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
pnpm --filter devx deploy --legacy --dev "$stage/devx-closure"
rm -f "$stage/devx-closure/node_modules/.pnpm/node_modules/devx"
cp -RL "$stage/devx-closure/node_modules" "$stage/support/node_modules"
rm -rf "$stage/devx-closure"

node_package_arch="$native_node_arch"
native_binding_libc=""
if [[ "$native_node_platform" == "linux" ]]; then
	native_binding_libc="$(native_linux_libc "$node_bin")"
fi
native_binding_suffix="$(native_binding_suffix "$native_node_platform" "$node_package_arch" "$native_binding_libc")" || {
	printf 'unsupported native binding target: %s/%s (%s)\n' "$native_node_platform" "$node_package_arch" "$native_binding_libc" >&2
	exit 1
}

resolve_package() {
	local package="$1"
	local parent="$2"

	"$node_bin" -e '
		const { createRequire } = require("node:module");
		const fs = require("node:fs");
		const path = require("node:path");
		const packageName = process.argv[1];
		const parentName = process.argv[2];
		const workspace = process.argv[3];
		const store = path.join(workspace, "..", "..", "node_modules", ".pnpm");
		let parent;
		try {
			parent = require.resolve(`${parentName}/package.json`, { paths: [workspace] });
		} catch {
			const encodedParent = parentName.replace("/", "+");
			const candidates = fs.readdirSync(store)
				.filter((entry) => entry.startsWith(encodedParent + "@"))
				.map((entry) => path.join(store, entry, "node_modules", parentName, "package.json"))
				.filter((candidate) => fs.existsSync(candidate));
			if (candidates.length !== 1) throw new Error(`unable to select locked parent ${parentName}`);
			parent = candidates[0];
		}
		const resolve = createRequire(parent);
		try {
			console.log(path.dirname(resolve.resolve(`${packageName}/package.json`)));
		} catch {
			const manifest = resolve(parentName + "/package.json");
			const version = manifest.dependencies?.[packageName] ?? manifest.optionalDependencies?.[packageName];
			if (!version || /[^0-9.]/.test(version)) throw new Error(`no exact locked version for ${packageName}`);
			const encoded = packageName.replace("/", "+");
			const entry = fs.readdirSync(store).find((name) => name === `${encoded}@${version}` || name.startsWith(`${encoded}@${version}_`));
			if (!entry) throw new Error(`pnpm store entry not found for ${packageName}@${version}`);
			console.log(path.join(store, entry, "node_modules", packageName));
		}
	' "$package" "$parent" "$REPO_ROOT/packages/devx"
}

copy_package() {
	local package="$1"
	local target="$2"
	local parent="$3"
	local source

	if ! source="$(resolve_package "$package" "$parent")" || [[ ! -d "$source" ]]; then
		printf 'locked runtime package is unavailable: %s\n' "$package" >&2
		exit 1
	fi

	mkdir -p "$(dirname "$target")"
	cp -RL "$source" "$target"
}

# pnpm deploy omits optional native packages. Resolve exact versions from the
# frozen workspace graph instead of selecting an arbitrary store wildcard.
copy_package esbuild "$stage/support/node_modules/esbuild" tsx
copy_package "@esbuild/$native_node_platform-$node_package_arch" "$stage/support/node_modules/@esbuild/$native_node_platform-$node_package_arch" esbuild
copy_package "@oxc-parser/binding-$native_binding_suffix" "$stage/support/node_modules/@oxc-parser/binding-$native_binding_suffix" oxc-parser
copy_package tinypool "$stage/support/node_modules/tinypool" oxfmt
copy_package "@oxfmt/binding-$native_binding_suffix" "$stage/support/node_modules/@oxfmt/binding-$native_binding_suffix" oxfmt
copy_package "@oxlint/binding-$native_binding_suffix" "$stage/support/node_modules/@oxlint/binding-$native_binding_suffix" oxlint

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
COPYFILE_DISABLE=1 tar -h -C "$stage" -czf "$archive" bin go lib support
go -C "$GO_WORKDIR" run ./packages/driver/cmd/runtime-manifest \
	--root "$stage" \
	--archive "$archive" \
	--output "$archive.manifest.json" \
	--required 'bin/node,bin/fmt-ts,bin/fmt-lint,bin/tsx,bin/oxfmt,bin/oxlint,go/bin/go,support/scripts/format-all.ts'

printf 'Wrote %s and %s.manifest.json\n' "$archive" "$archive"
