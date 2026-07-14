#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../../.." && pwd)"
actual_go="$(command -v go)"
tmp_root="$(mktemp -d)"

cleanup() {
	chmod -R u+w "$tmp_root" 2>/dev/null || true
	rm -rf "$tmp_root"
}

trap cleanup EXIT

write_file() {
	local path="$1"
	local content="$2"

	mkdir -p "$(dirname "$path")"
	printf '%s' "$content" >"$path"
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

assert_go_invocation_equals() {
	local fixture_root="$1"
	local expected_target="$2"
	local expected

	expected="-C $repo_root run ./packages/driver/cmd/fmtkit-go format --cwd $fixture_root $expected_target"

	assert_contains "$fixture_root/go-invocations.log" "$expected"
}

create_fixture() {
	local name="$1"
	local fixture_root
	fixture_root="$(cd "$tmp_root" && pwd -P)/$name"

	mkdir -p "$fixture_root"
	cp -R "$repo_root/cmd" "$fixture_root/cmd"
	cp -R "$repo_root/scripts" "$fixture_root/scripts"
	mkdir -p "$fixture_root/semantic"
	mkdir -p "$fixture_root/storage/test-bin"

	cat >"$fixture_root/oxfmt-stub.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf 'oxfmt %s\n' "$*" >> "${TOOL_WRAPPER_LOG:?}"
exit 0
EOF
	chmod +x "$fixture_root/oxfmt-stub.sh"

	cat >"$fixture_root/tsx-stub.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf 'tsx %s\n' "$*" >> "${TOOL_WRAPPER_LOG:?}"
exit 0
EOF
	chmod +x "$fixture_root/tsx-stub.sh"

	cat >"$fixture_root/storage/test-bin/go" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >> "\${GO_WRAPPER_LOG:?}"
exec "$actual_go" "\$@"
EOF
	chmod +x "$fixture_root/storage/test-bin/go"

	(
		cd "$fixture_root"
		git init -q
		git config user.email tests@example.com
		git config user.name 'Test Runner'
		git add cmd scripts oxfmt-stub.sh tsx-stub.sh
		git commit -q -m 'initial'
	)

	printf '%s\n' "$fixture_root"
}

run_format() {
	local fixture_root="$1"
	shift

	: >"$fixture_root/go-invocations.log"
	: >"$fixture_root/tool-invocations.log"

	(
		cd "$fixture_root"
			GO_BIN="$fixture_root/storage/test-bin/go" \
			GO_WRAPPER_LOG="$fixture_root/go-invocations.log" \
			GO_WORKDIR="$repo_root" \
			OXFMT_BIN="$fixture_root/oxfmt-stub.sh" \
			TOOL_WRAPPER_LOG="$fixture_root/tool-invocations.log" \
			TSX_BIN="$fixture_root/tsx-stub.sh" \
			./infra/scripts/tasks/format.sh "$@"
	)
}

test_tracked_go_uses_git_diff() {
	local fixture_root
	fixture_root="$(create_fixture tracked-go)"

	write_file "$fixture_root/semantic/pkg/api/changed.go" 'package sample

func run() {
	println("ok")
}
'

	(
		cd "$fixture_root"
		git add semantic/pkg/api/changed.go
		git commit -q -m 'add tracked file'
	)

	write_file "$fixture_root/semantic/pkg/api/changed.go" 'package sample

func run() {
	defer println("done")
	return
}
'

	run_format "$fixture_root" .

	assert_go_invocation_equals "$fixture_root" "$fixture_root"
	assert_contains "$fixture_root/semantic/pkg/api/changed.go" 'defer println("done")

	return'
}

test_untracked_go_disables_git_diff_and_formats_both() {
	local fixture_root
	fixture_root="$(create_fixture untracked-go)"

	write_file "$fixture_root/semantic/pkg/api/changed.go" 'package sample

func run() {
	println("ok")
}
'

	(
		cd "$fixture_root"
		git add semantic/pkg/api/changed.go
		git commit -q -m 'add tracked file'
	)

	write_file "$fixture_root/semantic/pkg/api/changed.go" 'package sample

func run() {
	defer println("done")
	return
}
'
	write_file "$fixture_root/semantic/pkg/api/new.go" 'package sample

func create() {
	defer println("new")
	return
}
'

	run_format "$fixture_root" .

	assert_go_invocation_equals "$fixture_root" "$fixture_root"
	assert_contains "$fixture_root/semantic/pkg/api/changed.go" 'defer println("done")

	return'
	assert_contains "$fixture_root/semantic/pkg/api/new.go" 'defer println("new")

	return'
}

test_untracked_non_go_keeps_git_diff() {
	local fixture_root
	fixture_root="$(create_fixture untracked-non-go)"

	write_file "$fixture_root/semantic/pkg/api/changed.go" 'package sample

func run() {
	println("ok")
}
'

	(
		cd "$fixture_root"
		git add semantic/pkg/api/changed.go
		git commit -q -m 'add tracked file'
	)

	write_file "$fixture_root/semantic/pkg/api/changed.go" 'package sample

func run() {
	defer println("done")
	return
}
'
	write_file "$fixture_root/semantic/pkg/api/notes.txt" 'keep me'

	run_format "$fixture_root" .

	assert_go_invocation_equals "$fixture_root" "$fixture_root"
	assert_contains "$fixture_root/semantic/pkg/api/changed.go" 'defer println("done")

	return'
}

test_explicit_path_uses_explicit_args() {
	local fixture_root
	fixture_root="$(create_fixture explicit-path)"

	write_file "$fixture_root/semantic/pkg/api/changed.go" 'package sample

func run() {
	println("ok")
}
'

	(
		cd "$fixture_root"
		git add semantic/pkg/api/changed.go
		git commit -q -m 'add tracked file'
	)

	write_file "$fixture_root/semantic/pkg/api/changed.go" 'package sample

func run() {
	defer println("done")
	return
}
'

	run_format "$fixture_root" semantic/pkg/api/changed.go

	assert_go_invocation_equals "$fixture_root" "$fixture_root/semantic/pkg/api/changed.go"
	assert_contains "$fixture_root/semantic/pkg/api/changed.go" 'defer println("done")

	return'
}

test_repo_root_falls_back_to_full_semantic_workspace() {
	local fixture_root
	fixture_root="$(create_fixture full-workspace-fallback)"

	write_file "$fixture_root/semantic/pkg/api/ordered.go" 'package sample

type Name string

const DefaultName Name = "demo"
'
	write_file "$fixture_root/semantic/pkg/api/changed.go" 'package sample

func run() {
	println("ok")
}
'

	(
		cd "$fixture_root"
		git add semantic/pkg/api/ordered.go semantic/pkg/api/changed.go
		git commit -q -m 'add tracked go files'
	)

	write_file "$fixture_root/semantic/pkg/api/changed.go" 'package sample

func run() {
	defer println("done")
	return
}
'

	run_format "$fixture_root" .

	assert_go_invocation_equals "$fixture_root" "$fixture_root"
	assert_contains "$fixture_root/semantic/pkg/api/changed.go" 'defer println("done")

	return'
}

test_ts_only_target_runs_full_pipeline_without_go_files() {
	local fixture_root
	fixture_root="$(create_fixture ts-only-target)"

	write_file "$fixture_root/src/app.ts" 'const value=1
'

	(
		cd "$fixture_root"
		git add "$fixture_root/src/app.ts"
		git commit -q -m 'add tracked ts file'
	)

	run_format "$fixture_root" "$fixture_root/src/app.ts"

	assert_go_invocation_equals "$fixture_root" "$fixture_root/src/app.ts"
	assert_contains "$fixture_root/go-invocations.log" "-C $repo_root run ./packages/driver/cmd/fmtkit-sources --cwd "
	assert_contains "$fixture_root/tool-invocations.log" "tsx $fixture_root/packages/devx/scripts/format-all.ts "
	assert_contains "$fixture_root/tool-invocations.log" "--oxfmt-bin $fixture_root/oxfmt-stub.sh"
	assert_contains "$fixture_root/tool-invocations.log" "$fixture_root/src/app.ts"
}

test_tracked_go_uses_git_diff
test_untracked_go_disables_git_diff_and_formats_both
test_untracked_non_go_keeps_git_diff
test_explicit_path_uses_explicit_args
test_repo_root_falls_back_to_full_semantic_workspace
test_ts_only_target_runs_full_pipeline_without_go_files
