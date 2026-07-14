#!/usr/bin/env bash
# shellcheck disable=SC1091
set -euo pipefail

task_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
source "$task_dir/env.sh"
source "$task_dir/runtime/common.sh"
source "$task_dir/runtime/platform.sh"

goos=''
goarch=''
binary=''
checksum=''
while (( $# > 0 )); do
	case "$1" in
		--goos)
			[[ -z "$goos" && "${2-}" != --* ]] || { printf 'invalid --goos argument\n' >&2; exit 2; }
			goos="$(require_nonblank --goos "${2-}")"; shift 2
			;;
		--goarch)
			[[ -z "$goarch" && "${2-}" != --* ]] || { printf 'invalid --goarch argument\n' >&2; exit 2; }
			goarch="$(require_nonblank --goarch "${2-}")"; shift 2
			;;
		--binary)
			[[ -z "$binary" && "${2-}" != --* ]] || { printf 'invalid --binary argument\n' >&2; exit 2; }
			binary="$(require_nonblank --binary "${2-}")"; shift 2
			;;
		--checksum)
			[[ -z "$checksum" && "${2-}" != --* ]] || { printf 'invalid --checksum argument\n' >&2; exit 2; }
			checksum="$(require_nonblank --checksum "${2-}")"; shift 2
			;;
		*) printf 'usage: verify-binary.sh --goos GOOS --goarch GOARCH --binary PATH --checksum PATH\n' >&2; exit 2 ;;
	esac
done

goos="$(require_nonblank --goos "$goos")"
goarch="$(require_nonblank --goarch "$goarch")"
binary="$(require_nonblank --binary "$binary")"
checksum="$(require_nonblank --checksum "$checksum")"
validate_runtime_platform "$goos" "$goarch"
binary="$(cd "$(dirname -- "$binary")" && pwd -P)/$(basename -- "$binary")"
require_no_extra_args "$@"
if [[ ! -f "$binary" || -L "$binary" || ! -x "$binary" ]]; then
	printf 'binary must be an executable regular file: %s\n' "$binary" >&2
	exit 1
fi
if [[ -e "$checksum" && -L "$checksum" ]]; then
	printf 'checksum must not be a symlink: %s\n' "$checksum" >&2
	exit 1
fi

mode="$(stat -f '%Lp' "$binary" 2>/dev/null || stat -c '%a' "$binary")"
if (( (8#$mode & 0077) != 0 )); then
	printf 'binary must not be group or other accessible: %s\n' "$binary" >&2
	exit 1
fi
file_description="$(file -b "$binary")"
case "$goos/$goarch:$file_description" in
	darwin/arm64:*arm64*) ;;
	linux/amd64:*x86-64* | linux/amd64:*x86_64*) ;;
	linux/arm64:*aarch64*) ;;
	*) printf 'binary architecture does not match %s/%s: %s\n' "$goos" "$goarch" "$file_description" >&2; exit 1 ;;
esac

verification_root="$(cd "$(mktemp -d)" && pwd -P)"
fixture="$verification_root/non-git"
runtime_dir="$verification_root/runtime"
clean_fixture="$verification_root/clean-git"
clean_runtime="$verification_root/clean-runtime"
default_fixture="$verification_root/default-git"
go_fixture="$verification_root/go-git"
ts_fixture="$verification_root/ts-git"
snapshots="$verification_root/snapshots"
cleanup() { rm -rf "$verification_root"; }
trap cleanup EXIT
mkdir -p "$fixture" "$runtime_dir" "$snapshots"
chmod 700 "$runtime_dir"

restricted_path='/usr/bin:/bin'
if ! PATH="$restricted_path" command -v git >/dev/null 2>&1; then
	printf 'git is required under the restricted PATH to verify implicit changed-file routing\n' >&2
	exit 1
fi

init_git_fixture() {
	local dir="$1"

	mkdir -p "$dir"
	printf 'module example.com/releasefixture\n\ngo 1.26.4\n' >"$dir/go.mod"
	printf 'package fixture\n\nfunc Changed() { println("baseline") }\n' >"$dir/changed.go"
	printf 'package fixture\n\nfunc Unchanged( ) { println("leave malformed") }\n' >"$dir/unchanged.go"
	printf 'export const changed = { value: 0 };\n' >"$dir/changed.ts"
	printf 'export const unchanged={value:0}\n' >"$dir/unchanged.ts"
	printf '<script setup lang="ts">\nconst message = "baseline";\n</script>\n<template><main>{{ message }}</main></template>\n' >"$dir/Changed.vue"
	printf '<script setup lang="ts">\nconst message="leave malformed"\n</script>\n<template><main>{{message}}</main></template>\n' >"$dir/Unchanged.vue"
	printf 'baseline\n' >"$dir/notes.md"
	git -C "$dir" init -q
	git -C "$dir" config user.email 'release-tests@example.com'
	git -C "$dir" config user.name 'Release Tests'
	git -C "$dir" add .
	git -C "$dir" -c commit.gpgsign=false commit -qm baseline
}

dirty_git_fixture() {
	local dir="$1"

	printf 'package fixture\n\nfunc Changed( ) { println("changed") }\n' >"$dir/changed.go"
	printf 'export const changed={value:1}\n' >"$dir/changed.ts"
	printf '<script setup lang="ts">\nconst message="changed"\n</script>\n<template><main>{{message}}</main></template>\n' >"$dir/Changed.vue"
	printf 'changed but unsupported\n' >"$dir/notes.md"
}

snapshot_fixture() {
	local name="$1"
	local dir="$2"
	local file

	mkdir -p "$snapshots/$name"
	for file in changed.go unchanged.go changed.ts unchanged.ts Changed.vue Unchanged.vue notes.md; do
		cp "$dir/$file" "$snapshots/$name/$file"
	done
}

assert_changed() {
	local name="$1"
	local dir="$2"
	local file="$3"

	if cmp -s "$snapshots/$name/$file" "$dir/$file"; then
		printf 'expected %s to be formatted in %s mode\n' "$file" "$name" >&2
		exit 1
	fi
}

assert_unchanged() {
	local name="$1"
	local dir="$2"
	local file="$3"

	if ! cmp -s "$snapshots/$name/$file" "$dir/$file"; then
		printf 'expected %s to remain untouched in %s mode\n' "$file" "$name" >&2
		exit 1
	fi
}

GO_FMT_RUNTIME_DIR="$runtime_dir" PATH="$restricted_path" "$binary" version

init_git_fixture "$clean_fixture"
(
	cd "$clean_fixture"
	GO_FMT_RUNTIME_DIR="$clean_runtime" PATH="$restricted_path" "$binary"
) >"$verification_root/clean.stdout" 2>"$verification_root/clean.stderr"
if [[ -e "$clean_runtime" ]]; then
	printf 'clean implicit Git run unexpectedly extracted the runtime\n' >&2
	exit 1
fi
if [[ -s "$verification_root/clean.stdout" || -s "$verification_root/clean.stderr" ]]; then
	printf 'clean implicit Git run must be a silent no-op\n' >&2
	exit 1
fi
if [[ -n "$(git -C "$clean_fixture" status --porcelain)" ]]; then
	printf 'clean implicit Git run modified tracked files\n' >&2
	exit 1
fi

init_git_fixture "$default_fixture"
dirty_git_fixture "$default_fixture"
snapshot_fixture default "$default_fixture"
(
	cd "$default_fixture"
	GO_FMT_RUNTIME_DIR="$runtime_dir" PATH="$restricted_path" "$binary"
)
for file in changed.go changed.ts Changed.vue; do
	assert_changed default "$default_fixture" "$file"
done
for file in unchanged.go unchanged.ts Unchanged.vue notes.md; do
	assert_unchanged default "$default_fixture" "$file"
done

init_git_fixture "$go_fixture"
dirty_git_fixture "$go_fixture"
snapshot_fixture go "$go_fixture"
(
	cd "$go_fixture"
	GO_FMT_RUNTIME_DIR="$runtime_dir" PATH="$restricted_path" "$binary" --go
)
assert_changed go "$go_fixture" changed.go
for file in changed.ts Changed.vue unchanged.go unchanged.ts Unchanged.vue notes.md; do
	assert_unchanged go "$go_fixture" "$file"
done

init_git_fixture "$ts_fixture"
dirty_git_fixture "$ts_fixture"
snapshot_fixture ts "$ts_fixture"
(
	cd "$ts_fixture"
	GO_FMT_RUNTIME_DIR="$runtime_dir" PATH="$restricted_path" "$binary" --ts
)
for file in changed.ts Changed.vue; do
	assert_changed ts "$ts_fixture" "$file"
done
for file in changed.go unchanged.go unchanged.ts Unchanged.vue notes.md; do
	assert_unchanged ts "$ts_fixture" "$file"
done

printf 'module example.com/containedfixture\n\ngo 1.26.4\n' >"$fixture/go.mod"
printf 'export const answer={value:1}\n' >"$fixture/example.ts"
printf '<script setup lang="ts">\nconst message="ok"\n</script>\n<template><main>{{ message }}</main></template>\n' >"$fixture/Example.vue"
printf 'package containedfixture\n\nfunc Example() { println("ok") }\n' >"$fixture/example.go"
(
	cd "$fixture"
	GO_FMT_RUNTIME_DIR="$runtime_dir" PATH="$restricted_path" "$binary" format .
	GO_FMT_RUNTIME_DIR="$runtime_dir" PATH="$restricted_path" "$binary" check .
)
if [[ -n "$(find "$runtime_dir" -type l -print -quit)" ]] || [[ -n "$(find "$runtime_dir" -perm -0077 -print -quit)" ]]; then
	printf 'private runtime cache contains unsafe permissions or symlinks\n' >&2
	exit 1
fi
portable_sha256 "$binary" >"$checksum"
