# Naming rules

`naming/no-redundant-name-prefix` reports a name that starts with what its path already says. It is off in the bundled policy, because it needs to know where your workspaces are. Enable it in `fmtkit.toml`:

```toml
[lint.rules]
"naming/no-redundant-name-prefix" = ["error", { workspaces = ["packages/*", "identity/auth"], terms = ["cloudflare-access"], folders = ["runtime-tests"], exempt = ["**/knock/workflows/**"] }]
```

The rule reads one authored TypeScript file at a time (`.ts` and `.tsx`, not `.d.ts`). A file outside every workspace is not judged.

## What the path says

The path says the workspace's folder name and the name of every folder between the workspace and the file. When workspaces nest, the innermost one counts. Each name counts in the plural and in the singular, so `notifications` also says `notification` and `policies` also says `policy`. Names are compared word by word, without case: kebab-case, snake_case, camelCase and PascalCase all split into words, so `magic-link`, `magicLink` and `MAGIC_LINK` are the same two words, and `http` does not match `https`.

| Name                                   | Judged against                         | Example under `packages/twilio/src/verify/`              |
| -------------------------------------- | -------------------------------------- | -------------------------------------------------------- |
| Folder inside a workspace              | the workspace and the folders above it | `verify/twilio-codes/` becomes `codes/`                  |
| File                                   | the workspace and every folder         | `twilio-verify-driver.ts` becomes `driver.ts`            |
| Module-level declaration, not exported | the workspace and every folder         | `const verifyTtl` becomes `ttl`                          |
| Exported declaration                   | the workspace only                     | `export class TwilioLookupDriver` becomes `LookupDriver` |

Prefixes that follow each other are trimmed together: `auth-sign-in-limit.ts` under `identity/auth/src/sign-in/` becomes `limit.ts`. The file name is judged without its extensions, so `notification-sink.test.ts` in `notifications` becomes `sink.test.ts`.

## What it keeps

- **A name that is only the prefix.** `provider.ts` in `providers` says something on its own.
- **An exported name that would be left with one word.** An export is read where it is imported, beside the workspace's import specifier but not its folders. `AccessProvider` in `access` keeps its name, because `Provider` alone would clash where the workspaces meet.
- **A configured term.** A trim that would cut a `terms` entry apart is not reported: with `terms = ["cloudflare-access"]`, `cloudflare-access-jwt.ts` in `cloudflare` stays. A term that is exactly the prefix does not protect it.
- **A configured folder name.** A `folders` entry, such as a folder a tool requires, is never reported. Files inside it are still judged.
- **An exempt path.** Files matching an `exempt` glob are skipped.

Nothing is exempt by default.

## When the shorter name is taken

When trimming a declaration gives a name the module already binds, by an import or another declaration, or two declarations trim to the same name, the message asks for a word that tells the two apart instead of the prefix. A name declared more than once, as a `const` and `type` pair, overloads or merged interfaces, is one entity and gets one finding. Keep the distinguishing word, not the domain. The rule sees one file at a time, so it cannot see a sibling file with the shorter name; apply the same choice when renaming files.

## Options

| Option       | Type           | Required | Meaning                                                                                 |
| ------------ | -------------- | -------- | --------------------------------------------------------------------------------------- |
| `workspaces` | array of globs | yes      | Workspace directories, relative to the repository root.                                 |
| `terms`      | array of names | no       | Product and vendor terms that a trim must not cut apart.                                |
| `folders`    | array of names | no       | Folder names that are never reported.                                                   |
| `exempt`     | array of globs | no       | Files the rule skips. A leading `!` re-includes a path; the last matching pattern wins. |

Globs follow the [`boundaries/*` rules](boundaries.md): a single `*` does not cross a `/`.
