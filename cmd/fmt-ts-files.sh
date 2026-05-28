#!/usr/bin/env bash
set -euo pipefail

support_dir="${GO_FMT_SUPPORT_DIR:-/opt/go-fmt/support}"
tsx_bin="${TSX_BIN:-${support_dir}/node_modules/.bin/tsx}"
oxfmt_bin="${OXFMT_BIN:-${support_dir}/node_modules/.bin/oxfmt}"
blank_lines_script="${GO_FMT_BLANK_LINES_SCRIPT:-${support_dir}/blank-lines.ts}"
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

while IFS= read -r -d '' file; do
	format_files+=("$file")
done < <(
	if [[ ${#sources_args[@]} -gt 0 ]]; then
		"${sources_cmd[@]}" "${sources_args[@]}" "${args[@]}"
	else
		"${sources_cmd[@]}" "${args[@]}"
	fi
)

while IFS= read -r -d '' file; do
	syntax_files+=("$file")
done < <(
	if [[ ${#sources_args[@]} -gt 0 ]]; then
		"${sources_cmd[@]}" "${sources_args[@]}" --include-declarations "${args[@]}"
	else
		"${sources_cmd[@]}" --include-declarations "${args[@]}"
	fi
)

if [[ ${#format_files[@]} -gt 0 ]]; then
	"$tsx_bin" "$blank_lines_script" "${format_files[@]}"
else
	"$tsx_bin" "$blank_lines_script"
fi

if [[ ${#format_files[@]} -gt 0 ]]; then
	printf '%s\0' "${format_files[@]}" \
		| xargs -0 "$oxfmt_bin" --write --no-error-on-unmatched-pattern
fi

if [[ ${#syntax_files[@]} -gt 0 ]]; then
	"$tsx_bin" "$validate_syntax_script" "${syntax_files[@]}"
else
	"$tsx_bin" "$validate_syntax_script"
fi
