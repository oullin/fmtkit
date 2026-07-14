#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

"${script_dir}/test-fmtkit-entrypoint.sh"
"${script_dir}/test-fmtkit-host-entrypoint.sh"
"${script_dir}/test-fmtkit-ts-entrypoint.sh"
"${script_dir}/test-fmtkit-lint-entrypoint.sh"
"${script_dir}/test-package-contained-runtime-lib.sh"
