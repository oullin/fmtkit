#!/usr/bin/env bash

go_arch_to_node_arch() {
	case "$1" in
		amd64) printf 'x64\n' ;;
		arm64) printf 'arm64\n' ;;
		*) return 1 ;;
	esac
}

validate_runtime_platform() {
	case "$1/$2" in
		darwin/arm64 | linux/amd64 | linux/arm64) ;;
		*)
			printf 'unsupported contained runtime platform: %s/%s\n' "$1" "$2" >&2
			return 1
			;;
	esac
}

native_linux_libc() {
	local node_bin="$1"
	"$node_bin" -p 'process.report.getReport().header.glibcVersionRuntime ? "gnu" : "musl"'
}

require_gnu_linux() {
	local node_bin="$1"
	local libc

	libc="$(native_linux_libc "$node_bin")"
	if [[ "$libc" != gnu ]]; then
		printf 'contained Linux runtimes require GNU libc; detected %s\n' "$libc" >&2
		return 1
	fi
}

validate_native_platform() {
	local goos="$1"
	local goarch="$2"
	local node_bin="$3"
	local native_goos native_goarch native_node_platform native_node_arch expected_node_arch

	native_goos="$(go -C "$GO_WORKDIR" env GOOS)"
	native_goarch="$(go -C "$GO_WORKDIR" env GOARCH)"
	native_node_platform="$("$node_bin" -p 'process.platform')"
	native_node_arch="$("$node_bin" -p 'process.arch')"
	expected_node_arch="$(go_arch_to_node_arch "$native_goarch")" || {
		printf 'unsupported Go architecture: %s\n' "$native_goarch" >&2
		return 1
	}

	if [[ "$goos" != "$native_goos" || "$goarch" != "$native_goarch" || "$goos" != "$native_node_platform" || "$expected_node_arch" != "$native_node_arch" ]]; then
		printf 'contained runtime packaging must run natively; requested %s/%s, Go is %s/%s, Node is %s/%s\n' "$goos" "$goarch" "$native_goos" "$native_goarch" "$native_node_platform" "$native_node_arch" >&2
		return 1
	fi

	if [[ "$goos" == linux ]]; then
		require_gnu_linux "$node_bin"
	fi
}
