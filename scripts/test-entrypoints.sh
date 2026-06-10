#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

"${script_dir}/test-fmt-all-entrypoint.sh"
"${script_dir}/test-fmt-ts-entrypoint.sh"
"${script_dir}/test-fmt-lint-entrypoint.sh"
