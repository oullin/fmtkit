#!/usr/bin/env bash
set -euo pipefail

usage() {
	printf 'usage: %s <go|ts> [args...]\n' "${0##*/}" >&2
	printf '  go [check|format|version|help] [args...]  run the Go formatter CLI\n' >&2
	printf '  ts [paths...]                            run TS/Vue blank-line support and oxfmt\n' >&2
}

if [[ $# -eq 0 ]]; then
	usage
	exit 2
fi

mode="$1"
shift

case "$mode" in
	go)
		exec /usr/local/bin/go-fmt "$@"
		;;
	ts)
		exec /usr/local/bin/format-ts "$@"
		;;
	*)
		usage
		exit 2
		;;
esac
