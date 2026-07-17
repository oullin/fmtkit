#!/usr/bin/env bash
set -euo pipefail

# Single entrypoint for the repo-wide tasks: everything that spans both halves
# of the pipeline lives here as a subcommand. Go-toolchain tasks scoped to a
# single package live in packages/go/infra/task.sh instead; the release tag
# machinery is under infra/release/.
#
# usage: task.sh <format|fmtkit|self-check|build|gofmt|coverage|with-env|help> [args...]

source "$(dirname "${BASH_SOURCE[0]}")/lib/env.sh"
source "$(dirname "${BASH_SOURCE[0]}")/lib/host-target.sh"

usage() {
	cat >&2 <<'EOF'
usage: task.sh <task> [args...]

  format [paths...]   format the repository with fmtkit's own binary; paths
                      are resolved against the repo root, "." means all of it
  fmtkit <cmd> ...    run any fmtkit command (format-all, check, version, ...)
                      through the same self-built binary
  self-check          assert the repo is fmtkit-formatted: format-all over a
                      clean tree and fail if anything moved
  build               build the local fmtkit-go binary into storage/bin
  gofmt               run gofmt -w over the Go module
  coverage            enforce the Go and TS coverage gates
  with-env <cmd> ...  run a command with the storage env and layout asserted
EOF
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

# The sidecar is stale once anything it is compiled from outdates it: the
# support sources, the tool pins, or the configs staged alongside it.
sidecar_is_stale() {
	local sidecar="$1"
	local newer

	[[ -x "$sidecar" ]] || return 0

	newer="$(find \
		"${REPO_ROOT}/packages/ts/sidecar/src" \
		"${REPO_ROOT}/packages/ts/sidecar/package.json" \
		"${REPO_ROOT}/.oxfmtrc.json" \
		"${REPO_ROOT}/.oxlintrc.json" \
		-newer "$sidecar" -print 2>/dev/null | head -n 1)"

	[[ -n "$newer" ]]
}

# Runs this repository through fmtkit's own binary — the same Go orchestrator
# and bun-compiled TS sidecar a release carries. The host toolchain assets are
# staged on demand and reused until their sources change. The repo root is not
# inside the Go module, so the inner loop is an incremental build into storage/
# rather than a `go run`; the embedded-asset path releases use is covered
# separately by infra/test-binary-smoke.sh.
run_fmtkit() {
	local support_dir sidecar bin

	support_dir="${REPO_ROOT}/packages/go/driver/internal/embedded/bin/$(host_target)"
	sidecar="${support_dir}/fmtkit-ts-sidecar"

	if sidecar_is_stale "$sidecar"; then
		"${REPO_ROOT}/packages/ts/infra/stage-ts-assets.sh" host
	fi

	ensure_storage_layout

	bin="$(canonical_path "${BUILD_DIR}/fmtkit-dev")"

	"${GO_BIN:-go}" -C "$GO_WORKDIR" build -o "$bin" ./driver/cmd/fmtkit

	cd "$REPO_ROOT"

	FMTKIT_SUPPORT_DIR="$support_dir" exec "$bin" "$@"
}

# Formats the repository. Paths are resolved against the repository root rather
# than the invoking directory, so `task.sh format .` means the whole repo no
# matter where it is run from.
run_format() {
	local -a args=("$@")
	local -a fmtkit_args=()
	local raw_arg

	if [[ "${args[0]:-}" == "--" ]]; then
		args=("${args[@]:1}")
	fi

	if [[ ${#args[@]} -eq 0 ]]; then
		args=(.)
	fi

	to_repo_path() {
		local arg="$1"

		case "$arg" in
			-*)
				# A step or output flag (--ts, --go, --quiet): pass it through as-is.
				printf '%s\n' "$arg"
				;;
			.)
				printf '%s\n' "$REPO_ROOT"
				;;
			./*)
				printf '%s\n' "$REPO_ROOT/${arg#./}"
				;;
			/*)
				printf '%s\n' "$arg"
				;;
			*)
				printf '%s\n' "$REPO_ROOT/$arg"
				;;
		esac
	}

	for raw_arg in "${args[@]}"; do
		fmtkit_args+=("$(to_repo_path "$raw_arg")")
	done

	run_fmtkit format "${fmtkit_args[@]}"
}

# Asserts this repository is formatted the way fmtkit formats it — by running
# fmtkit over it and failing if anything moved.
#
# This is the only honest way to check the tree. fmtkit's format is the
# pipeline's output (blank-lines -> oxfmt -> fluent-chains), and the project
# passes run *after* oxfmt and deliberately diverge from it: they expand calls
# and chains that oxfmt, left to itself, would collapse back. So bare
# `oxfmt --check` disagrees with correctly formatted source by design, and using
# it here would be checking the tree against a tool that is not the formatter.
#
# The pipeline has no read-only mode for TS, so this formats and then diffs.
# It is meant for CI and for a clean tree; it rewrites files in place.
# run_fmtkit ends in `exec`, so it runs in a subshell to keep this one alive for
# the diff.
run_self_check() {
	cd "$REPO_ROOT"

	if [[ -n "$(git status --porcelain)" ]]; then
		printf 'self-check: the working tree is dirty; commit or stash first\n' >&2
		git status --short >&2
		exit 1
	fi

	(run_fmtkit format-all)

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

run_build() {
	local host_os host_arch build_dir_path bin_path

	host_os="${HOST_OS:-$(go -C "$GO_WORKDIR" env GOOS)}"
	host_arch="${HOST_ARCH:-$(go -C "$GO_WORKDIR" env GOARCH)}"
	build_dir_path="$(canonical_path "$BUILD_DIR")"
	bin_path="$(canonical_path "$BIN")"

	ensure_storage_layout
	mkdir -p "$build_dir_path" "$(dirname "$bin_path")"

	CGO_ENABLED="$CGO_ENABLED" GOOS="$host_os" GOARCH="$host_arch" \
		go -C "$GO_WORKDIR" build -trimpath -ldflags "-s -w -X main.version=$VERSION" -o "$bin_path" "$CMD"
	chmod +x "$bin_path"
}

run_coverage() {
	local go_coverage

	with_env go -C "$GO_WORKDIR" test ./... -coverprofile=coverage.out -covermode=atomic

	go_coverage="$(go -C "$GO_WORKDIR" tool cover -func=coverage.out | awk '/^total:/ { gsub(/%/, "", $3); print $3 }')"

	awk -v coverage="${go_coverage}" 'BEGIN { exit !(coverage >= 88) }'

	printf 'Go coverage: %s%%\n' "${go_coverage}"

	with_env pnpm --filter sidecar run test:coverage
}

task="${1:-help}"
shift || true

case "$task" in
	format)
		run_format "$@"
		;;
	fmtkit)
		run_fmtkit "$@"
		;;
	self-check)
		run_self_check
		;;
	build)
		run_build
		;;
	gofmt)
		exec gofmt -w "$GO_WORKDIR"
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
