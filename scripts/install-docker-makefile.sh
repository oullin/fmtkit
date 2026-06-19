#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
template="$repo_root/tooling/docker/Makefile"
target="$repo_root/Makefile"

if [[ ! -f "$template" ]]; then
	printf 'docker Makefile template not found: %s\n' "$template" >&2
	exit 1
fi

if [[ -e "$target" && "${FORCE:-0}" != 1 ]]; then
	printf 'refusing to overwrite existing Makefile: %s\n' "$target" >&2
	printf 'Set FORCE=1 to replace it.\n' >&2
	exit 1
fi

cp "$template" "$target"
printf 'Installed Docker compatibility Makefile at %s\n' "$target"
