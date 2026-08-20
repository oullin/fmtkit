#!/usr/bin/env bash
set -euo pipefail

# Builds the linux/amd64 fmtkit binary, assembles a build context shaped like
# goreleaser dockers_v2's (prebuilt binaries under <os>/<arch>/ next to the
# Dockerfile), builds the image, and formats a bind-mounted fixture from inside
# it. Exercises the image's baked-in git safe.directory config (container root
# against a runner-owned mount) and the FMTKIT_SUPPORT_DIR short-circuit (a
# non-root run with no writable cache). Requires bash, git, go, node, npm,
# bun, and docker.

repo_root="$(cd "$(dirname "$0")/.." && pwd)"

"${repo_root}/packages/ts/toolchain/stage-ts-assets.sh" linux_amd64

tmp_root="$(mktemp -d)"

cleanup() {
	rm -rf "$tmp_root"
}

trap cleanup EXIT

ctx="${tmp_root}/ctx"

mkdir -p "${ctx}/linux/amd64"

(
	cd "$repo_root"

	CGO_ENABLED=0 GOOS=linux GOARCH=amd64 \
		go -C packages/go build -trimpath -tags fmtkit_sidecar \
		-o "${ctx}/linux/amd64/fmtkit" ./driver/cmd/fmtkit
)

cp "${repo_root}/Dockerfile" "${ctx}/Dockerfile"

docker buildx build --load --platform linux/amd64 -t fmtkit:smoke "$ctx"

# Same TS/Go probes as test-binary-smoke.sh: the double-quoted string checks
# that oxfmt picks up the bundled config, the Go file checks the in-process
# formatter.
fixture="${tmp_root}/fixture"

mkdir -p "$fixture"
cd "$fixture"

git init --quiet .

printf 'const  a = { x:1, s:"hi" }\nexport default a\n' > app.ts
printf 'package p\n\nfunc f() {\n\tdefer println("d")\n\treturn\n}\n' > app.go
printf 'module fixture\n\ngo 1.26.5\n' > go.mod

docker run --rm -v "${fixture}:/work" fmtkit:smoke format .

expected_ts=$'const a = { x: 1, s: \'hi\' };\n\nexport default a;\n'
expected_go=$'package p\n\nfunc f() {\n\tdefer println("d")\n\n\treturn\n}\n'

if ! diff <(printf '%s' "$expected_ts") app.ts; then
	printf 'app.ts was not formatted as expected inside the image\n' >&2
	exit 1
fi

if ! diff <(printf '%s' "$expected_go") app.go; then
	printf 'app.go was not formatted as expected inside the image\n' >&2
	exit 1
fi

# A non-root uid with no writable cache dir only works when the image's
# pre-extracted toolchain (FMTKIT_SUPPORT_DIR) is picked up.
docker run --rm -u "$(id -u):$(id -g)" -v "${fixture}:/work" fmtkit:smoke format .

printf 'docker smoke test passed\n'
