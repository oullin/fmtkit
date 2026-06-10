#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
tmp_root="$(mktemp -d)"

cleanup() {
	rm -rf "$tmp_root"
}

trap cleanup EXIT

support_dir="$tmp_root/support"
bin_dir="$tmp_root/bin"
workdir="$tmp_root/work"
log_file="$tmp_root/invocations.log"

mkdir -p "$support_dir/node_modules/.bin" "$bin_dir" "$workdir"
: >"$log_file"

write_executable() {
	local path="$1"
	local body="$2"

	printf '%s\n' "$body" >"$path"
	chmod +x "$path"
}

write_executable "$bin_dir/fmt-sources" '#!/usr/bin/env bash
set -euo pipefail
printf "fmt-sources %s\n" "$*" >> "'"$log_file"'"
printf "fmt-sources-config %s %s %s\n" "${GIT_CONFIG_COUNT:-}" "${GIT_CONFIG_KEY_0:-}" "${GIT_CONFIG_VALUE_0:-}" >> "'"$log_file"'"
printf "sample.ts\0"'

write_executable "$support_dir/node_modules/.bin/oxlint" '#!/usr/bin/env bash
set -euo pipefail
printf "oxlint %s\n" "$*" >> "'"$log_file"'"'

# Bundled config exists but no project-local config -> --config fallback is used.
touch "$support_dir/.oxlintrc.json"

(
	cd "$workdir"
	PATH="$bin_dir:$PATH" \
		GO_FMT_SUPPORT_DIR="$support_dir" \
		GO_FMT_SOURCES_BIN="$bin_dir/fmt-sources" \
		"$repo_root/cmd/fmt-lint" .
)

expected=$'fmt-sources .\nfmt-sources-config 1 safe.directory *\noxlint --config '"$support_dir"$'/.oxlintrc.json sample.ts'
actual="$(<"$log_file")"

if [[ "$actual" != "$expected" ]]; then
	printf 'unexpected invocation log\nexpected:\n%s\nactual:\n%s\n' "$expected" "$actual" >&2
	exit 1
fi

# A project-local config takes precedence: no --config flag is passed.
: >"$log_file"
touch "$workdir/.oxlintrc.json"

(
	cd "$workdir"
	PATH="$bin_dir:$PATH" \
		GO_FMT_SUPPORT_DIR="$support_dir" \
		GO_FMT_SOURCES_BIN="$bin_dir/fmt-sources" \
		"$repo_root/cmd/fmt-lint" .
)

expected=$'fmt-sources .\nfmt-sources-config 1 safe.directory *\noxlint sample.ts'
actual="$(<"$log_file")"

if [[ "$actual" != "$expected" ]]; then
	printf 'unexpected invocation log (project-local config)\nexpected:\n%s\nactual:\n%s\n' "$expected" "$actual" >&2
	exit 1
fi
