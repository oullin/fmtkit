#!/usr/bin/env bash

resolve_package() {
	local package="$1"
	local parent="$2"

	"$NODE_BIN" -e '
		const { createRequire } = require("node:module");
		const fs = require("node:fs");
		const path = require("node:path");
		const [packageName, parentName, workspace] = process.argv.slice(1);
		const store = path.join(workspace, "..", "..", "node_modules", ".pnpm");
		let parent;
		try { parent = require.resolve(`${parentName}/package.json`, { paths: [workspace] }); }
		catch {
			const encodedParent = parentName.replace("/", "+");
			const candidates = fs.readdirSync(store).filter((entry) => entry.startsWith(encodedParent + "@"))
				.map((entry) => path.join(store, entry, "node_modules", parentName, "package.json")).filter(fs.existsSync);
			if (candidates.length !== 1) throw new Error(`unable to select locked parent ${parentName}`);
			parent = candidates[0];
		}
		const resolve = createRequire(parent);
		try { console.log(path.dirname(resolve.resolve(`${packageName}/package.json`))); }
		catch {
			const manifest = require(resolve(parentName + "/package.json"));
			const version = manifest.dependencies?.[packageName] ?? manifest.optionalDependencies?.[packageName];
			if (!version || /[^0-9.]/.test(version)) throw new Error(`no exact locked version for ${packageName}`);
			const encoded = packageName.replace("/", "+");
			const entry = fs.readdirSync(store).find((name) => name === `${encoded}@${version}` || name.startsWith(`${encoded}@${version}_`));
			if (!entry) throw new Error(`pnpm store entry not found for ${packageName}@${version}`);
			console.log(path.join(store, entry, "node_modules", packageName));
		}
	' "$package" "$parent" "$REPO_ROOT/packages/devx"
}

copy_locked_package() {
	local package="$1"
	local target="$2"
	local parent="$3"
	local source

	if ! source="$(resolve_package "$package" "$parent")" || [[ ! -d "$source" ]]; then
		printf 'locked runtime package is unavailable: %s\n' "$package" >&2
		return 1
	fi

	mkdir -p "$(dirname "$target")"
	cp -RL "$source" "$target"
}
