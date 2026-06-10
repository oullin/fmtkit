#!/usr/bin/env bash
set -euo pipefail

support_dir="${GO_FMT_SUPPORT_DIR:-/opt/go-fmt/support}"
oxlint_bin="${OXLINT_BIN:-${support_dir}/node_modules/.bin/oxlint}"
oxlintrc="${GO_FMT_OXLINTRC:-${support_dir}/.oxlintrc.json}"

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

declare -a lint_files=()

# Capture the source listing through a temp file rather than process
# substitution: a `while … done < <(cmd)` loop swallows cmd's exit status, so a
# failing sources_cmd would silently yield an empty list and pass. Writing to a
# file lets set -e catch the failure.
tmp_sources="$(mktemp)"
trap 'rm -f "$tmp_sources"' EXIT

if [[ ${#sources_args[@]} -gt 0 ]]; then
	"${sources_cmd[@]}" "${sources_args[@]}" "${args[@]}" > "$tmp_sources"
else
	"${sources_cmd[@]}" "${args[@]}" > "$tmp_sources"
fi

while IFS= read -r -d '' file; do
	lint_files+=("$file")
done < "$tmp_sources"

rm -f "$tmp_sources"
trap - EXIT

if [[ ${#lint_files[@]} -eq 0 ]]; then
	echo "[lint] no TS/Vue files to lint."
	exit 0
fi

# Fall back to the bundled config only when the project has no oxlint config of
# its own; a project-local config (.oxlintrc.json, .jsonc, …) takes precedence
# via oxlint's native auto-discovery.
detect_dir="${GO_FMT_SOURCES_CWD:-$PWD}"
declare -a oxlint_config_args=()
# Match both the extensioned configs (.oxlintrc.json/.jsonc/…) and the
# extensionless .oxlintrc. nullglob strips the glob when nothing matches, but
# the literal .oxlintrc (no metacharacters) survives even when absent, so the
# -f guard below filters to configs that actually exist on disk.
shopt -s nullglob
declare -a project_configs=()
for candidate in "$detect_dir"/.oxlintrc "$detect_dir"/.oxlintrc.*; do
	[[ -f "$candidate" ]] && project_configs+=("$candidate")
done
shopt -u nullglob

if [[ ${#project_configs[@]} -eq 0 && -f "$oxlintrc" ]]; then
	oxlint_config_args=(--config "$oxlintrc")
fi

printf '%s\0' "${lint_files[@]}" \
	| xargs -0 "$oxlint_bin" ${oxlint_config_args[@]+"${oxlint_config_args[@]}"}
