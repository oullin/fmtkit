#!/usr/bin/env bash
set -euo pipefail

# Computes the next semver tag from commits since the latest v* tag,
# creates the tag, pushes it, and writes new_tag to GITHUB_OUTPUT.
#
# Bump rules (conventional commits, default patch):
#   - BREAKING CHANGE or a ! marker in the type/scope -> major
#   - feat:                                            -> minor
#   - anything else                                    -> patch

tag_prefix="${TAG_PREFIX:-v}"
default_bump="${DEFAULT_BUMP:-patch}"

git fetch --tags --force origin >/dev/null 2>&1 || true

latest_tag="$(git tag -l "${tag_prefix}*" --sort=-v:refname | head -n1 || true)"

if [ -z "${latest_tag}" ]; then
	major=0
	minor=0
	patch=0
	commit_range="HEAD"
else
	version="${latest_tag#"${tag_prefix}"}"
	if ! [[ "${version}" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
		printf 'latest tag %s does not parse as strict %sMAJOR.MINOR.PATCH semver; refusing to bump\n' "${latest_tag}" "${tag_prefix}" >&2
		exit 1
	fi
	major="${BASH_REMATCH[1]}"
	minor="${BASH_REMATCH[2]}"
	patch="${BASH_REMATCH[3]}"
	commit_range="${latest_tag}..HEAD"
fi

if ! git rev-parse --verify HEAD >/dev/null 2>&1; then
	printf 'no commits found on HEAD\n' >&2
	exit 1
fi

commits="$(git log --format='%B%n---END---' "${commit_range}" 2>/dev/null || true)"

bump="${default_bump}"
if [ -n "${commits}" ]; then
	if printf '%s' "${commits}" | grep -qiE '(^|\n)BREAKING[ -]CHANGE(:|!)' \
		|| printf '%s' "${commits}" | grep -qE '^[a-zA-Z]+(\([^)]+\))?!:'; then
		bump="major"
	elif printf '%s' "${commits}" | grep -qE '^feat(\([^)]+\))?:'; then
		bump="minor"
	elif printf '%s' "${commits}" | grep -qE '^(fix|perf|refactor|revert)(\([^)]+\))?:'; then
		bump="patch"
	fi
fi

case "${bump}" in
	major)
		major=$((major + 1))
		minor=0
		patch=0
		;;
	minor)
		minor=$((minor + 1))
		patch=0
		;;
	patch)
		patch=$((patch + 1))
		;;
	*)
		printf 'unsupported bump: %s\n' "${bump}" >&2
		exit 1
		;;
esac

new_tag="${tag_prefix}${major}.${minor}.${patch}"

if git rev-parse -q --verify "refs/tags/${new_tag}" >/dev/null; then
	printf 'tag %s already exists\n' "${new_tag}" >&2
	exit 1
fi

git tag "${new_tag}"
git push origin "refs/tags/${new_tag}"

printf 'created tag %s (bump=%s, previous=%s)\n' "${new_tag}" "${bump}" "${latest_tag:-<none>}"

if [ -n "${GITHUB_OUTPUT:-}" ]; then
	printf 'new_tag=%s\n' "${new_tag}" >>"${GITHUB_OUTPUT}"
	printf 'previous_tag=%s\n' "${latest_tag}" >>"${GITHUB_OUTPUT}"
	printf 'bump=%s\n' "${bump}" >>"${GITHUB_OUTPUT}"
fi
