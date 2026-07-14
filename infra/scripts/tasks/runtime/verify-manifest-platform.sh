#!/usr/bin/env bash
# shellcheck source-path=SCRIPTDIR
set -euo pipefail

task_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
# shellcheck source=common.sh
source "$task_dir/runtime/common.sh"
# shellcheck source=platform.sh
source "$task_dir/runtime/platform.sh"
# shellcheck source=archive.sh
source "$task_dir/runtime/archive.sh"

goos=''
goarch=''
manifest=''
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
		--manifest)
			[[ -z "$manifest" && "${2-}" != --* ]] || { printf 'invalid --manifest argument\n' >&2; exit 2; }
			manifest="$(require_nonblank --manifest "${2-}")"; shift 2
			;;
		*) printf 'usage: verify-manifest-platform.sh --goos GOOS --goarch GOARCH --manifest PATH\n' >&2; exit 2 ;;
	esac
done

goos="$(require_nonblank --goos "$goos")"
goarch="$(require_nonblank --goarch "$goarch")"
manifest="$(require_nonblank --manifest "$manifest")"
validate_runtime_platform "$goos" "$goarch"
if [[ ! -f "$manifest" || -L "$manifest" ]]; then
	printf 'manifest must be a regular file: %s\n' "$manifest" >&2
	exit 1
fi
if [[ "$(basename "$manifest")" != "runtime-${goos}-${goarch}.tar.gz.manifest.json" ]]; then
	printf 'runtime manifest name does not match requested platform %s/%s: %s\n' "$goos" "$goarch" "$manifest" >&2
	exit 1
fi
validate_runtime_manifest_platform "$manifest" "$goos" "$goarch"
