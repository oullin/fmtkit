#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
release_image_dir="${script_dir}/image"

source "${release_image_dir}/config.sh"
source "${release_image_dir}/assertions.sh"
source "${release_image_dir}/fixtures.sh"
source "${release_image_dir}/docker.sh"
source "${release_image_dir}/go.sh"
source "${release_image_dir}/node-ts.sh"
source "${release_image_dir}/full.sh"

verify_go_image "${NEW_TAG}-go"
verify_node_ts_image "${NEW_TAG}-node-ts"
verify_full_image "${NEW_TAG}-full"
