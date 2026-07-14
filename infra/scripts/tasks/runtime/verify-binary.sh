#!/usr/bin/env bash
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
	linux/amd64:*x86-64* | linux/amd64:*x86_64* | linux/amd64:*AMD\ x86-64*) ;;
	linux/arm64:*aarch64* | linux/arm64:*ARM\ aarch64*) ;;
	*) printf 'binary architecture does not match %s/%s: %s\n' "$goos" "$goarch" "$file_description" >&2; exit 1 ;;
esac

fixture="$(cd "$(mktemp -d)" && pwd -P)"
runtime_dir="$(cd "$(mktemp -d)" && pwd -P)"
cleanup() { rm -rf "$fixture" "$runtime_dir"; }
trap cleanup EXIT
printf 'module example.com/containedfixture\n\ngo 1.26.4\n' >"$fixture/go.mod"
printf 'export const answer={value:1}\n' >"$fixture/example.ts"
printf '<script setup lang="ts">\nconst message="ok"\n</script>\n<template><main>{{ message }}</main></template>\n' >"$fixture/Example.vue"
printf 'package containedfixture\n\nfunc Example() { println("ok") }\n' >"$fixture/example.go"
(
	cd "$fixture"
	GO_FMT_RUNTIME_DIR="$runtime_dir" PATH=/usr/bin:/bin "$binary" version
	GO_FMT_RUNTIME_DIR="$runtime_dir" PATH=/usr/bin:/bin "$binary" format .
	GO_FMT_RUNTIME_DIR="$runtime_dir" PATH=/usr/bin:/bin "$binary" check .
)
if [[ -n "$(find "$runtime_dir" -type l -print -quit)" ]] || [[ -n "$(find "$runtime_dir" -perm -0077 -print -quit)" ]]; then
	printf 'private runtime cache contains unsafe permissions or symlinks\n' >&2
	exit 1
fi
portable_sha256 "$binary" >"$checksum"
