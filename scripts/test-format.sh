#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
actual_go="$(command -v go)"
tmp_root="$(mktemp -d)"

cleanup() {
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

create_fixture() {
	local name="$1"
	local fixture_root="$tmp_root/$name"

	mkdir -p "$fixture_root"
	cp -R "$repo_root/scripts" "$fixture_root/scripts"
	cp -R "$repo_root/semantic" "$fixture_root/semantic"
	mkdir -p "$fixture_root/bin"

	cat >"$fixture_root/oxfmt-stub.sh" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
	chmod +x "$fixture_root/oxfmt-stub.sh"

	cat >"$fixture_root/tsx-stub.sh" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
	chmod +x "$fixture_root/tsx-stub.sh"

	cat >"$fixture_root/bin/go" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" >> "\${GO_WRAPPER_LOG:?}"
exec "$actual_go" "\$@"
EOF
	chmod +x "$fixture_root/bin/go"

	(
		cd "$fixture_root"
		git init -q
		git config user.email tests@example.com
		git config user.name 'Test Runner'
		git add scripts semantic oxfmt-stub.sh tsx-stub.sh bin/go
		git commit -q -m 'initial'
	)

	printf '%s\n' "$fixture_root"
}

run_format() {
	local fixture_root="$1"
	shift

	: >"$fixture_root/go-invocations.log"

	(
		cd "$fixture_root"
		PATH="$fixture_root/bin:$PATH" \
			GO_WRAPPER_LOG="$fixture_root/go-invocations.log" \
			OXFMT_BIN="$fixture_root/oxfmt-stub.sh" \
			TSX_BIN="$fixture_root/tsx-stub.sh" \
			./scripts/format.sh "$@"
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

	assert_contains "$fixture_root/go-invocations.log" 'format --cwd . --git-diff'
	assert_not_contains "$fixture_root/go-invocations.log" 'format --cwd . .'
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

	assert_not_contains "$fixture_root/go-invocations.log" '--git-diff'
	assert_contains "$fixture_root/go-invocations.log" 'format --cwd .'
	assert_not_contains "$fixture_root/go-invocations.log" 'format --cwd . .'
	assert_contains "$fixture_root/go-invocations.log" 'pkg/api/changed.go'
	assert_contains "$fixture_root/go-invocations.log" 'pkg/api/new.go'
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

	assert_contains "$fixture_root/go-invocations.log" 'format --cwd . --git-diff'
	assert_not_contains "$fixture_root/go-invocations.log" 'format --cwd . .'
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

	assert_not_contains "$fixture_root/go-invocations.log" '--git-diff'
	assert_not_contains "$fixture_root/go-invocations.log" 'format --cwd . .'
	assert_contains "$fixture_root/go-invocations.log" 'format --cwd . pkg/api/changed.go'
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

	assert_contains "$fixture_root/go-invocations.log" 'format --cwd . --git-diff'
	assert_contains "$fixture_root/go-invocations.log" 'check --cwd . .'
	assert_contains "$fixture_root/go-invocations.log" 'format --cwd . .'
	assert_contains "$fixture_root/semantic/pkg/api/changed.go" 'defer println("done")

	return'
	assert_contains "$fixture_root/semantic/pkg/api/ordered.go" 'const DefaultName Name = "demo"

type Name string'
}

test_tracked_go_uses_git_diff
test_untracked_go_disables_git_diff_and_formats_both
test_untracked_non_go_keeps_git_diff
test_explicit_path_uses_explicit_args
test_repo_root_falls_back_to_full_semantic_workspace
