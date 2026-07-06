#!/usr/bin/env bash

release_image_run() {
	docker run --rm "$@"
}

release_image_run_in_workdir() {
	local workdir="$1"
	local image="$2"

	shift 2
	docker run --rm -v "${workdir}:/work" -w /work "$image" "$@"
}
