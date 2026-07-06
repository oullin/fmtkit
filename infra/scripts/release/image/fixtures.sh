#!/usr/bin/env bash

declare -a release_image_tmpdirs=()

cleanup_release_image_tmpdirs() {
	local tmpdir

	for tmpdir in ${release_image_tmpdirs[@]+"${release_image_tmpdirs[@]}"}; do
		rm -rf "$tmpdir"
	done
}

trap cleanup_release_image_tmpdirs EXIT

create_release_image_tmpdir() {
	local tmpdir

	tmpdir="$(mktemp -d)"
	release_image_tmpdirs+=("$tmpdir")
	printf '%s\n' "$tmpdir"
}

write_go_spacing_fixture() {
	local tmpdir="$1"

	cat > "${tmpdir}/sample.go" <<'EOF'
package sample

func run() {
	if true {
		println("ok")
	}
	println("next")
}
EOF
}

write_node_ts_fixture() {
	local tmpdir="$1"

	git -C "$tmpdir" init -q

	cat > "${tmpdir}/sample.ts" <<'EOF'
const value={name:"demo"};
EOF
}
