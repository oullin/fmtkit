#!/usr/bin/env bash
set -euo pipefail

# Single entrypoint for the repository tasks. The release tag machinery lives
# under scripts/release/.
#
# usage: task.sh <task> [args...]

source "$(dirname "${BASH_SOURCE[0]}")/lib/env.sh"

usage() {
	cat >&2 <<'EOF_USAGE'
usage: task.sh <task> [args...]

  build               build fmtkit (release profile) and the Go helper for
                      this machine into storage/
  fmtkit <cmd> ...    run any fmtkit command through the self-built binary
  format [paths...]   format the repository with fmtkit's own binary; paths
                      are resolved against the repo root, "." means all of it
  self-check          assert the repo is fmtkit-formatted: format --all over a
                      clean tree and fail if anything moved
  lint                rustfmt, clippy, gofmt and go vet, all read-only
  test                the Rust workspace and Go helper test suites
  coverage            enforce the Rust and Go coverage gates
  with-env <cmd> ...  run a command with the storage env and layout asserted
EOF_USAGE
}

# Runs a command with the storage layout in place, then asserts the command did
# not scatter artifacts outside storage/.
with_env() {
	local status

	ensure_storage_layout
	set +e
	"$@"
	status=$?
	set -e
	assert_no_legacy_artifacts

	return "$status"
}

run_build() {
	ensure_storage_layout
	cargo build --manifest-path "${REPO_ROOT}/Cargo.toml" --release --locked -p fmtkit
	"${REPO_ROOT}/scripts/build-go-helper.sh"
}

# Runs fmtkit from this checkout, rebuilt incrementally first, against the
# helper built beside it.
run_fmtkit() {
	run_build >&2

	cd "$REPO_ROOT"

	FMTKIT_GO_HELPER="$(canonical_path "$GO_HELPER_DIR")/fmtkit-go-helper" exec "${CARGO_TARGET_DIR}/release/fmtkit" "$@"
}

# Formats the repository. Paths are resolved against the repository root rather
# than the invoking directory, so `task.sh format .` means the whole repo no
# matter where it is run from.
run_format() {
	local -a fmtkit_args=(--all)
	local arg

	for arg in "$@"; do
		case "$arg" in
			--) ;;
			-*) fmtkit_args+=("$arg") ;;
			.) ;;
			/*) fmtkit_args+=("$arg") ;;
			*) fmtkit_args+=("${REPO_ROOT}/${arg#./}") ;;
		esac
	done

	run_fmtkit format "${fmtkit_args[@]}"
}

# Asserts this repository is formatted the way fmtkit formats it, by running
# fmtkit over it and failing if anything moved. Unlike `fmtkit check`, this
# also proves that a format run over the tree is a no-op end to end.
# run_fmtkit ends in `exec`, so it runs in a subshell to keep this one alive for
# the diff.
run_self_check() {
	cd "$REPO_ROOT"

	if [[ -n "$(git status --porcelain)" ]]; then
		printf 'self-check: the working tree is dirty; commit first\n' >&2
		git status --short >&2
		exit 1
	fi

	(run_fmtkit format --all --no-cache)

	if git diff --quiet; then
		printf 'repository is fmtkit-formatted\n'
		exit 0
	fi

	printf '\nself-check: fmtkit reformatted the following; commit the result:\n' >&2
	git diff --name-only >&2
	printf '\n' >&2
	git --no-pager diff >&2

	exit 1
}

run_lint() {
	local unformatted

	with_env cargo fmt --manifest-path "${REPO_ROOT}/Cargo.toml" --all --check
	with_env cargo clippy --manifest-path "${REPO_ROOT}/Cargo.toml" --workspace --all-targets --locked -- -D warnings

	unformatted="$(gofmt -l "${REPO_ROOT}/go/helper")"

	if [[ -n "$unformatted" ]]; then
		printf 'gofmt: these files need formatting:\n%s\n' "$unformatted" >&2
		exit 1
	fi

	with_env go -C "${REPO_ROOT}/go/helper" vet ./...
}

run_test() {
	"${REPO_ROOT}/scripts/build-go-helper.sh"

	FMTKIT_GO_HELPER="$(canonical_path "$GO_HELPER_DIR")/fmtkit-go-helper" \
		with_env cargo test --manifest-path "${REPO_ROOT}/Cargo.toml" --workspace --locked

	with_env go -C "${REPO_ROOT}/go/helper" test -race ./...
}

# Both gates hold at 90% of lines. cargo-llvm-cov must be installed
# (`cargo install cargo-llvm-cov`).
run_coverage() {
	local go_coverage profile

	"${REPO_ROOT}/scripts/build-go-helper.sh"

	FMTKIT_GO_HELPER="$(canonical_path "$GO_HELPER_DIR")/fmtkit-go-helper" \
		with_env cargo llvm-cov --manifest-path "${REPO_ROOT}/Cargo.toml" --workspace --locked \
		--lcov --output-path "$(canonical_path "${CACHE_DIR}/lcov.info")" --fail-under-lines 90

	profile="$(canonical_path "${CACHE_DIR}/go-helper.cover.out")"

	with_env go -C "${REPO_ROOT}/go/helper" test ./... -coverprofile="$profile" -covermode=atomic

	go_coverage="$(go -C "${REPO_ROOT}/go/helper" tool cover -func="$profile" | awk '/^total:/ { gsub(/%/, "", $3); print $3 }')"

	printf 'Go helper coverage: %s%%\n' "${go_coverage}"

	awk -v coverage="${go_coverage}" 'BEGIN { exit !(coverage >= 90) }'
}

task="${1:-help}"
shift || true

case "$task" in
	build)
		run_build
		;;
	fmtkit)
		run_fmtkit "$@"
		;;
	format)
		run_format "$@"
		;;
	self-check)
		run_self_check
		;;
	lint)
		run_lint
		;;
	test)
		run_test
		;;
	coverage)
		run_coverage
		;;
	with-env)
		with_env "$@"
		;;
	help | --help | -h)
		usage
		;;
	*)
		printf 'unknown task: %s\n\n' "$task" >&2
		usage
		exit 1
		;;
esac
