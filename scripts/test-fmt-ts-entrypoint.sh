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

write_executable "$bin_dir/git" '#!/usr/bin/env bash
set -euo pipefail
printf "git %s\n" "$*" >> "'"$log_file"'"
printf "git-config %s %s %s\n" "${GIT_CONFIG_COUNT:-}" "${GIT_CONFIG_KEY_0:-}" "${GIT_CONFIG_VALUE_0:-}" >> "'"$log_file"'"
if [[ "${1:-}" == "config" ]]; then
	exit 0
fi
if [[ "${1:-}" == "ls-files" ]]; then
	printf "sample.ts\0"
	exit 0
fi
printf "unexpected git command: %s\n" "$*" >&2
exit 1'

write_executable "$support_dir/node_modules/.bin/tsx" '#!/usr/bin/env bash
set -euo pipefail
printf "tsx %s\n" "$*" >> "'"$log_file"'"
printf "tsx-config %s %s %s\n" "${GIT_CONFIG_COUNT:-}" "${GIT_CONFIG_KEY_0:-}" "${GIT_CONFIG_VALUE_0:-}" >> "'"$log_file"'"'

write_executable "$support_dir/node_modules/.bin/oxfmt" '#!/usr/bin/env bash
set -euo pipefail
printf "oxfmt %s\n" "$*" >> "'"$log_file"'"
while IFS= read -r -d "" file; do
	printf "oxfmt-file %s\n" "$file" >> "'"$log_file"'"
done'

touch "$support_dir/blank-lines.ts"

(
	cd "$workdir"
	PATH="$bin_dir:$PATH" \
		GO_FMT_SUPPORT_DIR="$support_dir" \
		"$repo_root/cmd/fmt-ts" .
)

expected=$'tsx '"$support_dir"$'/blank-lines.ts .\ntsx-config 1 safe.directory *\ngit ls-files --cached --others --exclude-standard -z -- .\ngit-config 1 safe.directory *\noxfmt --write --no-error-on-unmatched-pattern sample.ts'
actual="$(<"$log_file")"

if [[ "$actual" != "$expected" ]]; then
	printf 'unexpected invocation log\nexpected:\n%s\nactual:\n%s\n' "$expected" "$actual" >&2
	exit 1
fi
