#!/usr/bin/env bash

runtime_archive_path() {
	printf '%s/runtime-%s-%s.tar.gz\n' "$(runtime_assets_dir)" "$1" "$2"
}

validate_archive_members() {
	local archive="$1"
	local member
	local members_file

	members_file="$(mktemp)"

	gzip -t "$archive" || { rm -f "$members_file"; return 1; }
	tar -tzf "$archive" >"$members_file" || { rm -f "$members_file"; return 1; }
	for member in bin/node bin/fmt-ts bin/fmt-lint bin/tsx bin/oxfmt bin/oxlint go/bin/go support/scripts/format-all.ts; do
		if ! grep -Fxq -- "$member" "$members_file"; then
			printf 'runtime archive %s is missing required member %s\n' "$archive" "$member" >&2
			rm -f "$members_file"
			return 1
		fi
	done

	if ! awk -F/ 'NF == 0 || $1 == "" || ($1 != "bin" && $1 != "go" && $1 != "lib" && $1 != "support") { exit 1 }' "$members_file"; then
		printf 'runtime archive contains an unexpected top-level path: %s\n' "$archive" >&2
		rm -f "$members_file"
		return 1
	fi
	rm -f "$members_file"
}

validate_archive_inputs() {
	local archive="$1"
	local manifest="$2"
	local goos="$3"
	local goarch="$4"

	if [[ ! -f "$archive" || -L "$archive" ]]; then
		printf 'missing or unsafe contained runtime archive: %s\n' "$archive" >&2
		return 1
	fi
	if [[ ! -f "$manifest" || -L "$manifest" ]]; then
		printf 'missing or unsafe runtime manifest: %s\n' "$manifest" >&2
		return 1
	fi
	if [[ "$(basename "$archive")" != "runtime-${goos}-${goarch}.tar.gz" ]]; then
		printf 'runtime archive name does not match requested platform %s/%s: %s\n' "$goos" "$goarch" "$archive" >&2
		return 1
	fi
	if [[ "$(basename "$manifest")" != "runtime-${goos}-${goarch}.tar.gz.manifest.json" ]]; then
		printf 'runtime manifest name does not match requested platform %s/%s: %s\n' "$goos" "$goarch" "$manifest" >&2
		return 1
	fi
	validate_archive_members "$archive"
	validate_runtime_manifest "$archive" "$manifest" "$goos" "$goarch"
}

validate_runtime_manifest() {
	local archive="$1"
	local manifest="$2"
	local goos="$3"
	local goarch="$4"
	local archive_sha

	archive_sha="$(portable_sha256 "$archive" | awk '{print $1}')"
	validate_runtime_manifest_platform "$manifest" "$goos" "$goarch"
	node - "$manifest" "$archive_sha" <<'JS'
		const fs = require("node:fs");
		const crypto = require("node:crypto");
		const [manifestPath, archiveSHA] = process.argv.slice(2);
		const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
		const required = ["bin/node", "bin/fmt-ts", "bin/fmt-lint", "bin/tsx", "bin/oxfmt", "bin/oxlint", "go/bin/go", "support/scripts/format-all.ts"];
		if (!/^[a-f0-9]{64}$/.test(manifest.archive_sha256) || !/^[a-f0-9]{64}$/.test(manifest.tree_sha256)) throw new Error("invalid runtime manifest checksums");
		if (manifest.archive_sha256 !== archiveSHA) throw new Error("runtime archive hash does not match manifest");
		if (!Array.isArray(manifest.required) || manifest.required.some((path) => typeof path !== "string" || !path || path.startsWith("/") || path.includes("\\\\") || path.split("/").some((part) => !part || part === "." || part === ".."))) throw new Error("invalid runtime manifest member path");
		for (const path of required) if (!manifest.required.includes(path)) throw new Error(`runtime manifest is missing required member ${path}`);
JS
}

validate_runtime_manifest_platform() {
	local manifest="$1"
	local goos="$2"
	local goarch="$3"

	node - "$manifest" "$goos" "$goarch" <<'JS'
		const fs = require("node:fs");
		const [manifestPath, goos, goarch] = process.argv.slice(2);
		const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
		const supported = new Set(["darwin/arm64", "linux/amd64", "linux/arm64"]);
		if (typeof manifest.goos !== "string" || typeof manifest.goarch !== "string" || !manifest.goos.trim() || !manifest.goarch.trim()) throw new Error("runtime manifest platform fields must be nonblank strings");
		if (manifest.goos !== manifest.goos.trim() || manifest.goarch !== manifest.goarch.trim()) throw new Error("runtime manifest platform fields must be canonical");
		if (!supported.has(`${manifest.goos}/${manifest.goarch}`)) throw new Error(`unsupported runtime manifest platform ${manifest.goos}/${manifest.goarch}`);
		if (manifest.goos !== goos || manifest.goarch !== goarch) throw new Error(`runtime manifest platform ${manifest.goos}/${manifest.goarch} does not match requested platform ${goos}/${goarch}`);
JS
}
