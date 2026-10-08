# Upgrading oxc

fmtkit builds on [oxc](https://github.com/oxc-project/oxc) for parsing, semantic analysis, linting and formatting. `oxc_linter` and the formatters are not published to crates.io, so every oxc crate is a git dependency pinned to **one** oxc release tag. Upgrade them together, in a pull request of their own, and never mix tags.

## What is pinned

| Where                                   | What                                                                                                            |
| --------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `Cargo.toml` `[workspace.dependencies]` | every `oxc_*` crate, all at the same `tag`                                                                      |
| `Cargo.toml` `[patch.crates-io]`        | `oxc_allocator`, routed to the same tag so that `oxc-markdown-parser` (crates.io) shares the git allocator type |
| `Cargo.toml` `oxc-markdown-parser`      | the crates.io parser behind `oxc_formatter_markdown`; its version must accept the pinned allocator's API        |

The current tag is `oxlint_v1.86.0`, the release that shipped oxlint 1.86.0 and oxfmt 0.71.0.

## Where oxc reaches into fmtkit

- `crates/ts/src/format.rs`: maps `[ts.format]` onto `oxc_formatter` options, the same way `apps/oxfmt` maps `.oxfmtrc.json`.
- `crates/ts/src/embed*`: the embedded-language dispatcher, ported from `apps/oxfmt/src/core/embed/`.
- `crates/ts/src/fingerprint.rs`: the parse-equivalence hash; new AST node kinds need a case here.
- `crates/ts/src/complexity/`: the scorer walks the AST directly.
- `crates/lint/src/oxc_bridge.rs`: builds the `oxc_linter` configuration in memory. All `oxc_linter` API use stays in this file.
- `crates/lint/src/rules/`: the native rules visit `oxc_semantic` nodes.
- `crates/hosts/src/{css,markdown}.rs`: `oxc_formatter_css` and `oxc_formatter_markdown`.

## Checklist

1. Pick the new tag. Prefer an `oxlint_v*` tag; its release notes name the oxfmt version it ships.
2. Replace the tag on every oxc entry and in `[patch.crates-io]`, then run `cargo update` for the oxc crates. Check that `Cargo.lock` holds one oxc commit only:

    ```sh
    grep -o 'oxc-project/oxc?tag=[^#]*#[0-9a-f]*' Cargo.lock | sort -u
    ```

3. Fix compile errors. Use `apps/oxfmt` and `apps/oxlint` at the new tag as the reference for any API that moved.
4. Diff `apps/oxfmt/src/core/embed/` and `apps/oxfmt/src/core/options.rs` between the two tags. Port any routing or option-mapping change into `crates/ts`.
5. Run `./scripts/task.sh lint` and `./scripts/task.sh test`. Then deal with each kind of failure as follows:
    - **Lint policy** (`crates/lint/tests/policy.rs`): a rule was renamed, added or removed. Update `crates/lint/policy/oxlintrc.json` and the snapshot together, and record every rule that changed in the pull request.
    - **TS pass and pipeline tests**: `oxc_formatter` output moved. Decide whether the passes still produce the intended layout, rather than blessing the new output.
    - **Hosts goldens**: review the diff, then regenerate with `FMTKIT_BLESS=1 cargo test -p fmtkit-hosts --test goldens`.
    - **Complexity parity** (`fixtures/complexity/`): this must not move. A change here means the scorer depends on something oxc changed.
6. Run fmtkit over a large real repository before and after the upgrade. Compare the formatted trees and the lint reports. Every difference should be explained in the pull request.
7. Run `./scripts/task.sh self-check`, and commit whatever the new formatter rewrites in this repository in the same pull request.
