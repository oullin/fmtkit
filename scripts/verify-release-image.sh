#!/usr/bin/env bash
set -euo pipefail

if [ -z "${IMAGE_NAME:-}" ]; then
	printf 'IMAGE_NAME is required\n' >&2
	exit 1
fi

if [ -z "${NEW_TAG:-}" ]; then
	printf 'NEW_TAG is required\n' >&2
	exit 1
fi

verify_go_image() {
	local tag="$1"
	local image="${IMAGE_NAME}:${tag}"
	local version_output
	local tmpdir
	local report_output
	local report_status

	version_output="$(docker run --rm "$image" version)"

	if [ "$version_output" != "go-fmt ${NEW_TAG}" ]; then
		printf 'unexpected version output for %s: %s\n' "$image" "$version_output" >&2
		exit 1
	fi

	tmpdir="$(mktemp -d)"

	cat > "${tmpdir}/sample.go" <<'EOF'
package sample

func run() {
	if true {
		println("ok")
	}
	println("next")
}
EOF

	set +e
	report_output="$(docker run --rm -v "${tmpdir}:/work" -w /work "$image" check . 2>&1)"
	report_status=$?
	set -e

	printf '%s\n' "$report_output"

	if [ "$report_status" -ne 1 ]; then
		printf 'expected check to exit 1 for %s, got %s\n' "$image" "$report_status" >&2
		exit 1
	fi

	grep -Fq "  sample.go" <<<"$report_output"
	grep -Fq "    [spacing] line 7: missing blank line after if statement" <<<"$report_output"
	grep -Fq "  Result: fail. 1 changed, 1 violation(s), 0 error(s)." <<<"$report_output"

	if grep -Fq "~ sample.go:7 [spacing]" <<<"$report_output"; then
		printf 'legacy flat renderer detected in %s\n' "$image" >&2
		exit 1
	fi

	rm -rf "$tmpdir"
}

verify_node_ts_image() {
	local tag="$1"
	local image="${IMAGE_NAME}:${tag}"
	local tmpdir

	tmpdir="$(mktemp -d)"

	git -C "$tmpdir" init -q

	cat > "${tmpdir}/sample.ts" <<'EOF'
const value={name:"demo"};
EOF

	docker run --rm -v "${tmpdir}:/work" -w /work "$image" .

	grep -Fq 'const value = { name: "demo" };' "${tmpdir}/sample.ts"

	if docker run --rm "$image" go version >/dev/null 2>&1; then
		printf 'node-ts image unexpectedly accepts Go CLI commands: %s\n' "$image" >&2
		exit 1
	fi

	rm -rf "$tmpdir"
}

verify_full_image() {
	local tag="$1"
	local image="${IMAGE_NAME}:${tag}"
	local version_output

	version_output="$(docker run --rm "$image" version)"

	if [ "$version_output" != "go-fmt ${NEW_TAG}" ]; then
		printf 'unexpected version output for %s: %s\n' "$image" "$version_output" >&2
		exit 1
	fi

	verify_go_image "$tag"

	if [ "$(docker run --rm "$image" go version)" != "go-fmt ${NEW_TAG}" ]; then
		printf 'full image did not forward go version correctly: %s\n' "$image" >&2
		exit 1
	fi
}

verify_go_image "${NEW_TAG}-go"
verify_node_ts_image "${NEW_TAG}-node-ts"
verify_full_image "${NEW_TAG}-full"
