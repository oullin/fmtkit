#!/usr/bin/env bash
set -euo pipefail

# Go-toolchain tasks scoped to one package of the module. The package.json
# shims in driver/, formatter/, and vet/ call this from their own directory,
# so ./... means that package's tree. Repo-wide tasks live in infra/task.sh.
#
# usage: task.sh <check|test|vet|gofmt> [args...]

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"

source "${script_dir}/../../../infra/lib/env.sh"

with_env() {
	local status

	ensure_storage_layout
	set +e
	"$@"
	status=$?
	set -e
	assert_no_legacy_artifacts

	exit "$status"
}

task="${1:-}"
shift || true

case "$task" in
	check)
		with_env go test ./... "$@"
		;;
	test)
		with_env go test ./... -v "$@"
		;;
	vet)
		with_env go vet ./... "$@"
		;;
	gofmt)
		exec gofmt -w . "$@"
		;;
	*)
		printf 'usage: %s <check|test|vet|gofmt> [args...]\n' "${0##*/}" >&2
		exit 1
		;;
esac
