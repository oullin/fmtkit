#!/usr/bin/env bash
set -euo pipefail

# Asserts this repository is formatted the way fmtkit formats it — by running
# fmtkit over it and failing if anything moved.
#
# This is the only honest way to check the tree. fmtkit's format is the
# pipeline's output (blank-lines → oxfmt → fluent-chains), and the project
# passes run *after* oxfmt and deliberately diverge from it: they expand calls
# and chains that oxfmt, left to itself, would collapse back. So bare
# `oxfmt --check` disagrees with correctly formatted source by design, and using
# it here would be checking the tree against a tool that is not the formatter.
#
# The pipeline has no read-only mode for TS, so this formats and then diffs.
# It is meant for CI and for a clean tree; it rewrites files in place.

source "$(dirname "${BASH_SOURCE[0]}")/env.sh"

cd "$REPO_ROOT"

if [[ -n "$(git status --porcelain)" ]]; then
	printf 'check-self-formatted: the working tree is dirty; commit or stash first\n' >&2
	git status --short >&2
	exit 1
fi

"${REPO_ROOT}/infra/scripts/tasks/fmtkit.sh" format-all

if git diff --quiet; then
	printf 'repository is fmtkit-formatted\n'
	exit 0
fi

printf '\ncheck-self-formatted: fmtkit reformatted the following; commit the result:\n' >&2
git diff --name-only >&2
printf '\n' >&2
git --no-pager diff >&2

exit 1
