#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

task="${1:-}"
shift || true

case "$task" in
	check)
		exec "$script_dir/with-storage-env.sh" go test ./... "$@"
		;;
	test)
		exec "$script_dir/with-storage-env.sh" go test ./... -v "$@"
		;;
	vet)
		exec "$script_dir/with-storage-env.sh" go vet ./... "$@"
		;;
	gofmt)
		exec gofmt -w . "$@"
		;;
	*)
		printf 'usage: %s <check|test|vet|gofmt> [args...]\n' "${0##*/}" >&2
		exit 1
		;;
esac
