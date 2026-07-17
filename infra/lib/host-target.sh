#!/usr/bin/env bash

# Maps the running machine onto one of the <goos>_<goarch> names that the staged
# TS toolchain assets are keyed by. Sourced by stage-ts-assets.sh to pick what to
# build, and by tasks/fmtkit.sh to find what was built.

host_target() {
	local os arch

	case "$(uname -s)" in
		Darwin) os='darwin' ;;
		Linux) os='linux' ;;
		*)
			printf 'unsupported host OS: %s\n' "$(uname -s)" >&2
			return 1
			;;
	esac

	case "$(uname -m)" in
		arm64 | aarch64) arch='arm64' ;;
		x86_64) arch='amd64' ;;
		*)
			printf 'unsupported host arch: %s\n' "$(uname -m)" >&2
			return 1
			;;
	esac

	printf '%s_%s' "${os}" "${arch}"
}
