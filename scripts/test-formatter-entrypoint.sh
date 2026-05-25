#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
tmp_root="$(mktemp -d)"

cleanup() {
	rm -rf "$tmp_root"
}

trap cleanup EXIT

log_file="$tmp_root/invocations.log"
stdout_file="$tmp_root/stdout.log"
stderr_file="$tmp_root/stderr.log"

write_stub() {
	local path="$1"
	local name="$2"

	cat >"$path" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s %s\n' "$name" "\$*" >> "$log_file"
case "$name" in
	format-ts)
		printf '[blank-lines] processed 3 file(s) in /work, 0 changed\n'
		printf 'Finished in 10ms on 3 files using 8 threads.\n'
		;;
	go-fmt)
		if [[ "\${1:-}" == "format" ]]; then
			printf '\nFormatter\n\n'
			printf '  Formatted 2 file(s).\n\n'
			printf '  Result: pass. 0 changed, 0 violation(s), 0 error(s).\n\n'
			printf 'Vet\n\n'
			printf '  go vet ./... passed.\n\n'
			printf '  Result: pass. 0 error(s).\n'
		else
			printf 'go-fmt output\n'
		fi
		;;
esac
EOF
	chmod +x "$path"
}

assert_contains() {
	local path="$1"
	local needle="$2"
	local content

	content="$(<"$path")"

	if [[ "$content" != *"$needle"* ]]; then
		printf 'expected %s to contain %q\n' "$path" "$needle" >&2
		exit 1
	fi
}

assert_not_contains() {
	local path="$1"
	local needle="$2"
	local content

	content="$(<"$path")"

	if [[ "$content" == *"$needle"* ]]; then
		printf 'expected %s to not contain %q\n' "$path" "$needle" >&2
		exit 1
	fi
}

assert_log_equals() {
	local expected="$1"
	local content

	content="$(<"$log_file")"

	if [[ "$content" != "$expected" ]]; then
		printf 'unexpected invocation log\nexpected:\n%s\nactual:\n%s\n' "$expected" "$content" >&2
		exit 1
	fi
}

run_entrypoint() {
	: >"$log_file"
	: >"$stdout_file"
	: >"$stderr_file"

	GO_FMT_BIN="$tmp_root/go-fmt-stub" \
		FORMAT_TS_BIN="$tmp_root/format-ts-stub" \
		"$repo_root/scripts/formatter-entrypoint.sh" "$@" >"$stdout_file" 2>"$stderr_file"
}

write_stub "$tmp_root/go-fmt-stub" go-fmt
write_stub "$tmp_root/format-ts-stub" format-ts

run_entrypoint format .
assert_log_equals $'format-ts .\ngo-fmt format .'
assert_contains "$stderr_file" '==> Formatting target(s)'
assert_contains "$stderr_file" 'paths        .'
assert_contains "$stderr_file" '==> Running TS/Vue formatting'
assert_contains "$stderr_file" 'blank-lines  processed 3 file(s) in /work, 0 changed'
assert_contains "$stderr_file" 'oxfmt        Finished in 10ms on 3 files using 8 threads.'
assert_contains "$stderr_file" '==> Running Go formatting'
assert_contains "$stderr_file" 'go-fmt'
assert_contains "$stderr_file" 'Formatted 2 file(s).'
assert_contains "$stderr_file" 'result'
assert_contains "$stderr_file" 'pass. 0 changed, 0 violation(s), 0 error(s).'
assert_contains "$stderr_file" 'vet'
assert_contains "$stderr_file" 'go vet ./... passed.'
assert_contains "$stderr_file" '==> Formatting complete'
assert_contains "$stderr_file" 'status'
assert_contains "$stderr_file" 'done'

run_entrypoint format-all
assert_log_equals $'format-ts .\ngo-fmt format .'

run_entrypoint go format .
assert_log_equals 'go-fmt format .'
assert_not_contains "$stderr_file" '==> Running TS/Vue formatting'

run_entrypoint ts .
assert_log_equals 'format-ts .'
assert_not_contains "$stderr_file" '==> Running Go formatting'

run_entrypoint check .
assert_log_equals 'go-fmt check .'

run_entrypoint version
assert_log_equals 'go-fmt version'

if run_entrypoint unknown; then
	printf 'expected unknown mode to fail\n' >&2
	exit 1
fi

assert_contains "$stderr_file" 'usage: formatter-entrypoint.sh <format|format-all|go|ts|check|version|help> [args...]'
