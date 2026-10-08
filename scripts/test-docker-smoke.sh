#!/usr/bin/env bash
set -euo pipefail

# Builds the linux image the way publish-docker.yml does (prebuilt binaries
# under <os>/<arch>/ next to the Dockerfile) and formats a bind-mounted fixture
# from inside it, as root and as a non-root uid.
#
# usage: test-docker-smoke.sh [fmtkit-x86_64-unknown-linux-gnu.tar.xz]
#   Without an archive, cross-builds fmtkit with cargo for the matching linux
#   target (the rustup target and a linker must be installed). Requires bash,
#   git, go, and docker.

source "$(dirname "$0")/lib/env.sh"

case "$(uname -m)" in
	arm64 | aarch64) arch=arm64 triple=aarch64-unknown-linux-gnu ;;
	*) arch=amd64 triple=x86_64-unknown-linux-gnu ;;
esac

tmp_root="$(mktemp -d)"

cleanup() {
	rm -rf "$tmp_root"
}

trap cleanup EXIT

ctx="${tmp_root}/ctx"
platform_dir="${ctx}/linux/${arch}"

mkdir -p "$platform_dir"

if (($# == 1)); then
	tar -xJf "$1" -C "$platform_dir" --strip-components 1
else
	cargo build --manifest-path "${REPO_ROOT}/Cargo.toml" --release --locked -p fmtkit --target "$triple"
	cp "${CARGO_TARGET_DIR}/${triple}/release/fmtkit" "$platform_dir/"
	"${REPO_ROOT}/scripts/build-go-helper.sh" "$triple"
	cp "$(canonical_path "$GO_HELPER_DIR")/fmtkit-go-helper" "$platform_dir/"
fi

cp "${REPO_ROOT}/Dockerfile" "${ctx}/Dockerfile"

docker buildx build --load --platform "linux/${arch}" -t fmtkit:smoke "$ctx"

fixture="${tmp_root}/fixture"

mkdir -p "$fixture"
git -C "$fixture" init --quiet .

printf 'const  a = { x:1, s:"hi" }\nexport default a\n' > "${fixture}/app.ts"
printf 'package p\n\nfunc f() {\n\tdefer println("d")\n\treturn\n}\n' > "${fixture}/app.go"
printf 'module fixture\n\ngo 1.27.1\n' > "${fixture}/go.mod"

# The fixture is owned by the runner, not by the container's root: git must
# still read it.
docker run --rm -v "${fixture}:/work" fmtkit:smoke format --all

expected_ts=$'const a = { s: \'hi\', x: 1 };\n\nexport default a;\n'
expected_go=$'package p\n\nfunc f() {\n\tdefer println("d")\n\n\treturn\n}\n'

if ! diff <(printf '%s' "$expected_ts") "${fixture}/app.ts"; then
	printf 'app.ts was not formatted as expected inside the image\n' >&2
	exit 1
fi

if ! diff <(printf '%s' "$expected_go") "${fixture}/app.go"; then
	printf 'app.go was not formatted as expected inside the image\n' >&2
	exit 1
fi

# A non-root uid needs the image's HOME=/tmp for its cache and go vet.
docker run --rm -u "$(id -u):$(id -g)" -v "${fixture}:/work" fmtkit:smoke check --all

printf 'docker smoke test passed\n'
