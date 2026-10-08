#!/usr/bin/env bash
set -euo pipefail

# Tags HEAD as v<version>, the workspace version in Cargo.toml, pushes the tag,
# and writes new_tag to GITHUB_OUTPUT. cargo-dist refuses a tag that differs
# from the package version, so the version is bumped in Cargo.toml (in the
# commit being released) rather than derived here. Exits 0 without tagging
# when that version is already tagged; refuses a version that does not sort
# above the latest v* tag.

source "$(dirname "$0")/../lib/env.sh"

if ! [[ "${VERSION}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
	printf 'Cargo.toml version %s is not strict MAJOR.MINOR.PATCH semver; refusing to tag\n' "${VERSION}" >&2
	exit 1
fi

new_tag="v${VERSION}"

git fetch --tags --force origin > /dev/null 2>&1 || true

if git rev-parse -q --verify "refs/tags/${new_tag}" > /dev/null; then
	printf '%s is already tagged; bump the version in Cargo.toml to release\n' "${new_tag}"
	exit 0
fi

latest_tag="$(git tag -l 'v*' --sort=-v:refname | head -n1 || true)"

if [[ -n "${latest_tag}" ]] && [[ "$(printf '%s\n%s\n' "${latest_tag}" "${new_tag}" | sort -V | tail -n1)" != "${new_tag}" ]]; then
	printf 'version %s does not sort above the latest tag %s; refusing to tag\n' "${new_tag}" "${latest_tag}" >&2
	exit 1
fi

git tag "${new_tag}"
git push origin "refs/tags/${new_tag}"

printf 'created tag %s (previous=%s)\n' "${new_tag}" "${latest_tag:-<none>}"

if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
	printf 'new_tag=%s\n' "${new_tag}" >> "${GITHUB_OUTPUT}"
	printf 'previous_tag=%s\n' "${latest_tag}" >> "${GITHUB_OUTPUT}"
fi
