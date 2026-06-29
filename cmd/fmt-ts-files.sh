#!/usr/bin/env bash
set -euo pipefail

support_dir="${GO_FMT_SUPPORT_DIR:-/opt/go-fmt/support}"
tsx_bin="${TSX_BIN:-${support_dir}/node_modules/.bin/tsx}"
oxfmt_bin="${OXFMT_BIN:-${support_dir}/node_modules/.bin/oxfmt}"
oxfmtrc="${GO_FMT_OXFMTRC:-${support_dir}/.oxfmtrc.json}"
blank_lines_script="${GO_FMT_BLANK_LINES_SCRIPT:-${support_dir}/blank-lines.ts}"
fluent_chains_script="${GO_FMT_FLUENT_CHAINS_SCRIPT:-${support_dir}/fluent-chains.ts}"
validate_syntax_script="${GO_FMT_VALIDATE_SYNTAX_SCRIPT:-${support_dir}/validate-syntax.ts}"

declare -a sources_cmd

if [[ -n "${GO_FMT_SOURCES_BIN:-}" ]]; then
	sources_cmd=("${GO_FMT_SOURCES_BIN}")
elif [[ -x /usr/local/bin/fmt-sources ]]; then
	sources_cmd=(/usr/local/bin/fmt-sources)
else
	sources_cmd=("${GO_BIN:-go}" -C "${GO_FMT_SOURCES_GO_WORKDIR:-packages/driver}" run "${GO_FMT_SOURCES_CMD:-./cmd/fmt-sources}")
fi

declare -a sources_args=()

if [[ -n "${GO_FMT_SOURCES_CWD:-}" ]]; then
	sources_args=(--cwd "${GO_FMT_SOURCES_CWD}")
fi

declare -a args=("$@")

if [[ ${#args[@]} -eq 0 ]]; then
	args=(.)
fi

declare -a format_files=()
declare -a syntax_files=()

# Capture the source listings through temp files rather than process
# substitution: a `while … done < <(cmd)` loop swallows cmd's exit status, so a
# failing sources_cmd would silently yield an empty list and pass. Writing to a
# file lets set -e catch the failure.
tmp_format="$(mktemp)"
tmp_syntax="$(mktemp)"
trap 'rm -f "$tmp_format" "$tmp_syntax"' EXIT

if [[ ${#sources_args[@]} -gt 0 ]]; then
	"${sources_cmd[@]}" "${sources_args[@]}" "${args[@]}" > "$tmp_format"
	"${sources_cmd[@]}" "${sources_args[@]}" --include-declarations "${args[@]}" > "$tmp_syntax"
else
	"${sources_cmd[@]}" "${args[@]}" > "$tmp_format"
	"${sources_cmd[@]}" --include-declarations "${args[@]}" > "$tmp_syntax"
fi

while IFS= read -r -d '' file; do
	format_files+=("$file")
done < "$tmp_format"

while IFS= read -r -d '' file; do
	syntax_files+=("$file")
done < "$tmp_syntax"

rm -f "$tmp_format" "$tmp_syntax"
trap - EXIT

if [[ ${#format_files[@]} -gt 0 ]]; then
	"$tsx_bin" "$blank_lines_script" "${format_files[@]}"
else
	"$tsx_bin" "$blank_lines_script"
fi

# Fall back to the bundled config only when the project has no oxfmt config of
# its own; a project-local config (.oxfmtrc.json, .ts, .js, …) takes precedence
# via oxfmt's native auto-discovery.
detect_dir="${GO_FMT_SOURCES_CWD:-$PWD}"
declare -a oxfmt_config_args=()
shopt -s nullglob
project_configs=("$detect_dir"/.oxfmtrc.*)
shopt -u nullglob

if [[ ${#project_configs[@]} -eq 0 && -f "$oxfmtrc" ]]; then
	oxfmt_config_args=(--config "$oxfmtrc")
fi

run_oxfmt() {
	if [[ ${#format_files[@]} -gt 0 ]]; then
		printf '%s\0' "${format_files[@]}" \
			| xargs -0 "$oxfmt_bin" ${oxfmt_config_args[@]+"${oxfmt_config_args[@]}"} --write --no-error-on-unmatched-pattern
	fi
}

run_oxfmt

if [[ ${#format_files[@]} -gt 0 ]]; then
	"$tsx_bin" "$fluent_chains_script" "${format_files[@]}"
else
	"$tsx_bin" "$fluent_chains_script"
fi

run_oxfmt

if [[ ${#syntax_files[@]} -gt 0 ]]; then
	"$tsx_bin" "$validate_syntax_script" "${syntax_files[@]}"
else
	"$tsx_bin" "$validate_syntax_script"
fi
