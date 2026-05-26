#!/usr/bin/env bash
set -euo pipefail

declare -a makefiles=("$@")

if [[ ${#makefiles[@]} -eq 0 ]]; then
	makefiles=(Makefile)
fi

reset=$'\033[0m'
bold=$'\033[1m'
title_color=$'\033[1;32m'
target_color=$'\033[1;36m'
variable_color=$'\033[1;35m'
example_color=$'\033[32m'

declare -a targets=()
declare -a variables=()

while IFS=$'\t' read -r kind first second third; do
	case "$kind" in
		T)
			targets+=("$first"$'\t'"$second")
			;;
		V)
			variables+=("$first"$'\t'"$second"$'\t'"$third")
			;;
	esac
done < <(
	awk '
		BEGIN {
			FS = "## ";
		}
		function trim(value) {
			gsub(/^[[:space:]]+|[[:space:]]+$/, "", value);
			return value;
		}
		/^[a-zA-Z0-9_.-]+:.*## / {
			split($1, parts, ":");
			printf "T\t%s\t%s\n", parts[1], $2;
			next;
		}
		/^[A-Z_][A-Z0-9_]*[[:space:]]*\?*=.*## / {
			split($1, parts, "\\?=");
			printf "V\t%s\t%s\t%s\n", trim(parts[1]), trim(parts[2]), $2;
		}
	' "${makefiles[@]}"
)

printf '%b%s%b\n\n' "$title_color" "go-fmt developer targets" "$reset"
printf '%bTargets:%b\n' "$bold" "$reset"

for entry in "${targets[@]}"; do
	IFS=$'\t' read -r name description <<<"$entry"
	printf -v padded_name '%-14s' "$name"
	printf '  %b%s%b %s\n' "$target_color" "$padded_name" "$reset" "$description"
done

if [[ ${#variables[@]} -gt 0 ]]; then
	printf '\n%bVariables:%b\n' "$bold" "$reset"

	for entry in "${variables[@]}"; do
		IFS=$'\t' read -r name value description <<<"$entry"
		printf -v padded_name '%-18s' "$name"
		printf '  %b%s%b %-24s %s\n' "$variable_color" "$padded_name" "$reset" "$value" "$description"
	done
fi

printf '\n%bExamples:%b\n' "$bold" "$reset"
printf '  %b%s%b\n' "$example_color" "./scripts/with-storage-env.sh pnpm turbo run check --cache-dir=storage/.cache/turbo --filter=driver" "$reset"
printf '  %b%s%b\n' "$example_color" "./scripts/with-storage-env.sh pnpm turbo run check --cache-dir=storage/.cache/turbo --filter=devx" "$reset"
printf '  %b%s%b\n' "$example_color" "make gofmt" "$reset"
