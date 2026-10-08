#!/usr/bin/env bash
set -euo pipefail

# Times two fmtkit builds against a pinned public corpus and fails when the
# second is more than BENCH_TOLERANCE percent slower than the first in either
# scenario:
#
#   cold   check --all --no-cache   every file parsed, formatted, linted, scored
#   warm   check --all              every outcome served from the cache
#
# Both builds run in the same job on the same corpus, so runner noise cancels
# out of the ratio. Each build is a directory holding fmtkit and
# fmtkit-go-helper.
#
# usage: bench.sh <base-dir> <head-dir>
# Requires bash, git, and hyperfine.

source "$(dirname "$0")/lib/env.sh"

if (($# != 2)); then
	printf 'usage: bench.sh <base-dir> <head-dir>\n' >&2
	exit 2
fi

base="$(cd "$1" && pwd -P)"
head="$(cd "$2" && pwd -P)"
repo="${BENCH_REPO:-https://github.com/go-gitea/gitea}"
ref="${BENCH_REF:-v1.27.3}"
tolerance="${BENCH_TOLERANCE:-10}"
runs="${BENCH_RUNS:-10}"
corpus="$(canonical_path "${CACHE_DIR}/bench/$(basename "$repo")-${ref}")"
results="$(canonical_path "${CACHE_DIR}/bench/results")"

if [[ ! -d "${corpus}/.git" ]]; then
	git clone --quiet --depth 1 --branch "$ref" "$repo" "$corpus"
fi

mkdir -p "$results"

# Prints the mean of a hyperfine JSON export, in seconds.
mean() {
	sed -n 's/^ *"mean": \([0-9.e+-]*\),$/\1/p' "$1" | head -n1
}

failed=0

for scenario in cold warm; do
	case "$scenario" in
		cold) args='check --all --no-cache --quiet' ;;
		warm) args='check --all --quiet' ;;
	esac

	for side in base head; do
		dir="${!side}"

		# check exits 1 on findings; only the time matters here.
		hyperfine --ignore-failure --warmup 1 --runs "$runs" --export-json "${results}/${scenario}-${side}.json" \
			--command-name "${side} ${scenario}" \
			"cd '${corpus}' && FMTKIT_GO_HELPER='${dir}/fmtkit-go-helper' FMTKIT_CACHE_DIR='${results}/cache-${side}' '${dir}/fmtkit' ${args}"
	done

	base_mean="$(mean "${results}/${scenario}-base.json")"
	head_mean="$(mean "${results}/${scenario}-head.json")"

	if awk -v b="$base_mean" -v h="$head_mean" -v t="$tolerance" 'BEGIN { exit !(h > b * (1 + t / 100)) }'; then
		printf '%s: head %.3fs is more than %s%% slower than base %.3fs\n' "$scenario" "$head_mean" "$tolerance" "$base_mean" >&2
		failed=1
	else
		printf '%s: head %.3fs, base %.3fs (tolerance %s%%)\n' "$scenario" "$head_mean" "$base_mean" "$tolerance"
	fi
done

exit "$failed"
