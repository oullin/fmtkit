# anti-slop (vendored)

Opinionated Oxlint JS-plugin rules that reject low-evidence TypeScript patterns.

- Upstream: https://github.com/dmmulroy/anti-slop
- Pinned commit: `446268e5d15baa968eaec669ff65358d36ae6259`
- Source path within upstream: `skills/install-anti-slop/assets/anti-slop/` (the tests-free bundle)

These rules lint fmtkit's own TypeScript only, loaded through the repo-root
`.oxlintrc.dev.json` used by the sidecar `lint`/`lint:check` scripts. They are
deliberately absent from `.oxlintrc.json`, which ships inside the release
binary as the fallback config for user projects.

This directory is excluded from the repo lint (`ignorePatterns` in
`.oxlintrc.dev.json`) because several rules would fire on their own
implementation. The files are formatted by fmtkit itself, so they diverge
byte-wise from upstream; when syncing a newer upstream, re-copy the bundle,
update the pinned commit above, and re-run `./scripts/task.sh format`.
