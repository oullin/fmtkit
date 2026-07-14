#!/usr/bin/env bash

go_arch_to_node_arch() {
	case "$1" in
		amd64) printf 'x64\n' ;;
		arm64) printf 'arm64\n' ;;
		*) return 1 ;;
	esac
}

native_binding_suffix() {
	local platform="$1"
	local arch="$2"
	local libc="${3:-}"

	case "$platform" in
		darwin)
			printf '%s-%s\n' "$platform" "$arch"
			;;
		linux)
			case "$libc" in
				gnu|musl) printf '%s-%s-%s\n' "$platform" "$arch" "$libc" ;;
				*) return 1 ;;
			esac
			;;
		*) return 1 ;;
	esac
}

native_linux_libc() {
	local node_bin="$1"
	"$node_bin" -p 'process.report.getReport().header.glibcVersionRuntime ? "gnu" : "musl"'
}
