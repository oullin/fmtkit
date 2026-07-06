#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"

"${script_dir}/with-storage-env.sh" go -C "${repo_root}/packages/formatter" test ./... -coverprofile=coverage.out -covermode=atomic

go_coverage="$(go -C "${repo_root}/packages/formatter" tool cover -func=coverage.out | awk '/^total:/ { gsub(/%/, "", $3); print $3 }')"

awk -v coverage="${go_coverage}" 'BEGIN { exit !(coverage >= 90) }'

printf 'formatter Go coverage: %s%%\n' "${go_coverage}"

"${script_dir}/with-storage-env.sh" pnpm --filter devx run test:coverage
