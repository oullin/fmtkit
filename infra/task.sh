#!/usr/bin/env bash
set -euo pipefail

# Single entrypoint for the repo-wide tasks: everything that spans both halves
# of the pipeline lives here as a subcommand. Go-toolchain tasks scoped to a
# single package live in packages/go/infra/task.sh instead; the release tag
# machinery is under infra/release/.
#
# usage: task.sh <format|fmtkit|build|gofmt|coverage|with-env|help> [args...]

source "$(dirname "${BASH_SOURCE[0]}")/lib/env.sh"
source "$(dirname "${BASH_SOURCE[0]}")/lib/host-target.sh"

usage() {
	cat >&2 <<'EOF'
usage: task.sh <task> [args...]

  format [paths...]   format the repository with fmtkit's own binary; paths
                      are resolved against the repo root, "." means all of it
  fmtkit <cmd> ...    run any fmtkit command (format-all, check, version, ...)
                      through the same self-built binary
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
