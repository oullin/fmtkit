#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../../.." && pwd)"
tmp_root="$(mktemp -d)"

cleanup() {
	rm -rf "$tmp_root"
}

trap cleanup EXIT

bin_dir="$tmp_root/bin"
project_dir="$tmp_root/project"
log_file="$tmp_root/docker.log"

mkdir -p "$bin_dir" "$project_dir"

cat >"$bin_dir/docker" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" > "$log_file"
EOF

chmod +x "$bin_dir/docker"

assert_contains() {
	local path="$1"
	local needle="$2"
	local content

	content="$(<"$path")"

	if [[ "$content" != *"$needle"* ]]; then
		printf 'expected %s to contain %q\n' "$path" "$needle" >&2
		printf 'actual:\n%s\n' "$content" >&2
		exit 1
	fi
}

(
	cd "$tmp_root"
	PATH="$bin_dir:$PATH" \
		FMTKIT_IMAGE="ghcr.io/oullin/fmtkit:test" \
		FMTKIT_CACHE_VOLUME="fmtkit-cache-test" \
		FMTKIT_PROJECT_DIR="project" \
		"$repo_root/infra/scripts/tasks/fmtkit-host.sh" format .
)

assert_contains "$log_file" "run --rm"
assert_contains "$log_file" "-v $project_dir:/work"
assert_contains "$log_file" "-v fmtkit-cache-test:/cache"
assert_contains "$log_file" "-e HOST_PROJECT_PATH=$project_dir"
assert_contains "$log_file" "ghcr.io/oullin/fmtkit:test format ."
