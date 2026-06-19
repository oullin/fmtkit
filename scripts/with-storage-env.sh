#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/env.sh"

ensure_storage_layout
set +e
"$@"
status=$?
set -e
assert_no_legacy_artifacts
exit "$status"
