# Naming rules

Three opt-in rules keep names short:

- [`naming/no-redundant-name-prefix`](#namingno-redundant-name-prefix) (TypeScript): a name that starts with what its path already says.
- [`naming/max-name-words`](#namingmax-name-words) (TypeScript and Go): a declared name with more words than a limit.
- [`naming/no-package-stutter`](#go) (Go): an exported name that starts with its package name.

All of them are off by default. The TypeScript rules are set in `[lint.rules]`; the Go checks in `[go]`.

## naming/no-redundant-name-prefix

`naming/no-redundant-name-prefix` reports a name that starts with what its path already says. It is off in the bundled policy, because it needs to know where your workspaces are. Enable it in `fmtkit.toml`:

```toml
[lint.rules]
"naming/no-redundant-name-prefix" = ["error", { workspaces = ["packages/*", "identity/auth"], terms = ["cloudflare-access"], folders = ["runtime-tests"], exempt = ["**/knock/workflows/**"] }]
```

The rule reads one authored TypeScript file at a time (`.ts` and `.tsx`, not `.d.ts`). A file outside every workspace is not judged.

### What the path says

The path says the workspace's folder name and the name of every folder between the workspace and the file. When workspaces nest, the innermost one counts. Each name counts in the plural and in the singular, so `notifications` also says `notification` and `policies` also says `policy`. Names are compared word by word, without case: kebab-case, snake_case, camelCase and PascalCase all split into words, so `magic-link`, `magicLink` and `MAGIC_LINK` are the same two words, and `http` does not match `https`.

| Name                                   | Judged against                         | Example under `packages/twilio/src/verify/`              |
| -------------------------------------- | -------------------------------------- | -------------------------------------------------------- |
| Folder inside a workspace              | the workspace and the folders above it | `verify/twilio-codes/` becomes `codes/`                  |
| File                                   | the workspace and every folder         | `twilio-verify-driver.ts` becomes `driver.ts`            |
| Module-level declaration, not exported | the workspace and every folder         | `const verifyTtl` becomes `ttl`                          |
| Exported declaration                   | the workspace only                     | `export class TwilioLookupDriver` becomes `LookupDriver` |

Prefixes that follow each other are trimmed together: `auth-sign-in-limit.ts` under `identity/auth/src/sign-in/` becomes `limit.ts`. The file name is judged without its extensions, so `notification-sink.test.ts` in `notifications` becomes `sink.test.ts`.

### What it keeps

- **A name that is only the prefix.** `provider.ts` in `providers` says something on its own.
- **An exported name that would be left with one word.** An export is read where it is imported, beside the workspace's import specifier but not its folders. `AccessProvider` in `access` keeps its name, because `Provider` alone would clash where the workspaces meet.
- **A configured term.** A trim that would cut a `terms` entry apart is not reported: with `terms = ["cloudflare-access"]`, `cloudflare-access-jwt.ts` in `cloudflare` stays. A term that is exactly the prefix does not protect it.
- **A configured folder name.** A `folders` entry, such as a folder a tool requires, is never reported. Files inside it are still judged.
- **An allowed name.** A folder, file stem or declaration matching an `allow` entry is never reported. An entry is a whole name or uses `*` for any run of characters, so `*ServiceProvider` keeps every workspace's service provider. Use it for names kept on purpose, such as role names that would clash across workspaces.
- **An exempt path.** Files matching an `exempt` glob are skipped.

Nothing is exempt by default.

### When the shorter name is taken

When trimming a declaration gives a name the module already binds, by an import or another declaration, or two declarations trim to the same name, the message asks for a word that tells the two apart instead of the prefix. A name declared more than once, as a `const` and `type` pair, overloads or merged interfaces, is one entity and gets one finding. Keep the distinguishing word, not the domain. The rule sees one file at a time, so it cannot see a sibling file with the shorter name; apply the same choice when renaming files.

### Options

| Option       | Type           | Required | Meaning                                                                                 |
| ------------ | -------------- | -------- | --------------------------------------------------------------------------------------- |
| `workspaces` | array of globs | yes      | Workspace directories, relative to the repository root.                                 |
| `terms`      | array of names | no       | Product and vendor terms that a trim must not cut apart.                                |
| `folders`    | array of names | no       | Folder names that are never reported.                                                   |
| `allow`      | array of names | no       | Folder, file and declaration names never reported; `*` matches any characters.          |
| `exempt`     | array of globs | no       | Files the rule skips. A leading `!` re-includes a path; the last matching pattern wins. |

Globs follow the [`boundaries/*` rules](boundaries.md): a single `*` does not cross a `/`.

## naming/max-name-words

`naming/max-name-words` reports a declared name with more words than `max`, 2 by default. A long name usually carries context that something around it could carry instead: a module, namespace or class whose name makes the long name unnecessary.

```toml
[lint.rules]
"naming/max-name-words" = ["error", { max = 2, allow = ["*ServiceProvider"], exempt = ["src/generated/**"] }]
```

Words are counted the way `naming/no-redundant-name-prefix` splits names: kebab-case, snake_case, camelCase and PascalCase all split into words, and an uppercase run ends before the word it leads, so `HTTPServer` is `HTTP` and `Server`. Digits stay with their word: `oauth2Client` is two words.

| Name                                    | Words | With `max = 2` |
| --------------------------------------- | ----- | -------------- |
| `QueueRouter`                           | 2     | kept           |
| `correlationId`                         | 2     | kept           |
| `workerEnv`                             | 2     | kept           |
| `BackgroundExceptionBoundary`           | 3     | reported       |
| `ObservedExecutionContext`              | 3     | reported       |
| `runWithAuthBackgroundScope`            | 5     | reported       |
| `readBookingProposalTtlMs`              | 5     | reported       |
| `DEFAULT_AUTH_SESSION_LIFETIME_SECONDS` | 5     | reported       |

### What it judges

A name is judged where it is declared, never where it is used or imported: functions, classes, interfaces, type aliases, enums and their members, namespaces, variables and constants at module level and inside functions, parameters (including those in function types and interface method signatures), catch bindings, class methods, properties and accessors, and the file name without its extensions (`booking-proposal-ttl.ts` is three words). A name declared more than once, as overloads, merged interfaces or a getter and setter, is reported once.

The rule reads every script fmtkit lints, `.js` as well as `.ts`. File names are judged for scripts only: Vue's own style guide asks for multi-word component names.

### What it leaves alone

Names the author does not choose freely:

- **Imported bindings.** `import { someImportedHelper }` is the exporter's name.
- **Shorthand destructuring.** `const { bindingName } = x` takes the property's name; a renamed `const { a: myLongName } = x` is judged.
- **Object-literal keys, interface and type-literal members.** They describe the shape of data.
- **Type parameters.**
- **Members of a class that `extends` or `implements` something.** The rule has no types, so it cannot tell which members the parent requires. The class's own name, and the parameters and locals inside its methods, are still judged.
- **Ambient declarations.** Anything marked `declare`, everything inside `declare module`, `module "name"` and `declare global`, `declare` class fields, and `.d.ts`, `.d.mts` and `.d.cts` files.
- **Test files**, unless `ignoreTests` is `false`.

### Options

| Option        | Type           | Default | Meaning                                                                                 |
| ------------- | -------------- | ------- | --------------------------------------------------------------------------------------- |
| `max`         | integer, ≥ 1   | `2`     | The most words a name may have.                                                         |
| `allow`       | array of names | `[]`    | Names and file stems never reported; `*` matches any characters.                        |
| `exempt`      | array of globs | `[]`    | Files the rule skips. A leading `!` re-includes a path; the last matching pattern wins. |
| `ignoreTests` | boolean        | `true`  | Skip test files, as the [`anti-slop-martinfowler/*` rules](anti-slop.md) do.            |

## Go

The Go lane runs both checks in `fmtkit-go-helper`, on the text it formatted, and reports them as lint errors under the same rule names. They are off by default:

```toml
[go]
max_name_words = true      # naming/max-name-words
max_words = 2              # package-level names, methods, struct fields, interface methods
max_local_words = 1        # locals and parameters; unset, max_words
no_package_stutter = true  # naming/no-package-stutter
```

The Go limit is independent of the TypeScript rule's `max`, so Go can run at 1 while TypeScript stays at 2.

### naming/max-name-words in Go

The rule judges the identifiers a file declares, each against one of two limits:

| Limit             | Names                                                                                                              |
| ----------------- | ------------------------------------------------------------------------------------------------------------------ |
| `max_words`       | package-level funcs, types, vars and consts; methods; struct fields; interface methods                             |
| `max_local_words` | variables, consts and types declared inside a function; parameters, named results and receivers, closures included |

Go carries context at the call site, so the message points there: a package-level name is read with its package (`archive.Pack`, not `release.PackDirectory`), and a method with its receiver type (`(*Archive).Limit`, not `MaxArchiveBytes`). A local lives in a short scope, where the function, its receiver and its package already say the rest.

Words split as in TypeScript, and Go initialisms are one word, plural or not: `userID` and `userIDs` are two words, `ServeHTTP` is two. The initialisms are golint's list (`ACL`, `API`, `ASCII`, `CPU`, `CSS`, `CSV`, `DNS`, `EOF`, `GUID`, `HTML`, `HTTP`, `HTTPS`, `ID`, `IP`, `JSON`, `JWT`, `LHS`, `QPS`, `RAM`, `RHS`, `RPC`, `SLA`, `SMTP`, `SQL`, `SSH`, `TCP`, `TLS`, `TTL`, `UDP`, `UI`, `UID`, `URI`, `URL`, `UTF8`, `UUID`, `VM`, `XML`, `XMPP`, `XSRF`, `XSS`).

Some words are Go idiom rather than description, and are not counted, so a limit of 1 stays workable:

| Idiom                     | Where                                                                                                                                                                   | Counts as        |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- |
| a leading `New` or `Must` | a function (a constructor), not a method or a variable                                                                                                                  | `NewServer`: 1   |
| a leading `Is` or `Has`   | a predicate: a function or method returning one `bool`, a var, field or parameter of type `bool`, or a local set to a comparison, `!x`, `&&`, `\|\|`, `true` or `false` | `IsReady`: 1     |
| the `Err` prefix          | a package-level variable (a sentinel error)                                                                                                                             | `ErrNotFound`: 2 |
| the `Err` suffix          | a local variable or parameter                                                                                                                                           | `closeErr`: 1    |

Names the author does not choose freely are not judged: `_`; the `init` and `main` functions; functions and methods go test runs, `Test*`, `Benchmark*`, `Fuzz*` and `Example*` (where the prefix ends the name or is followed by a non-lowercase letter, so `Testify` is judged); imports; type parameters; labels; embedded fields; parameter names in function types; and methods, including interface methods, named for a standard library interface. Only names of more than one word can be over a limit, so that list holds only those:

| Interface                                       | Methods                                                                                                                      |
| ----------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| `fmt.GoStringer`                                | `GoString`                                                                                                                   |
| `io` readers and writers                        | `ReadAt`, `WriteAt`, `ReadFrom`, `WriteTo`, `ReadByte`, `UnreadByte`, `WriteByte`, `ReadRune`, `UnreadRune`, `WriteString`   |
| `encoding`                                      | `MarshalText`, `UnmarshalText`, `AppendText`, `MarshalBinary`, `UnmarshalBinary`, `AppendBinary`                             |
| `encoding/json`, `encoding/xml`, `encoding/gob` | `MarshalJSON`, `UnmarshalJSON`, `MarshalXML`, `UnmarshalXML`, `MarshalXMLAttr`, `UnmarshalXMLAttr`, `GobEncode`, `GobDecode` |
| `net/http`                                      | `ServeHTTP`, `RoundTrip`                                                                                                     |
| `log/slog`                                      | `LogValue`, `WithAttrs`, `WithGroup`                                                                                         |

Test files are judged like any other Go file. A local redeclared with `:=` is judged once per top-level declaration.

### naming/no-package-stutter

An exported package-level name (a func, type, var or const) that starts with its package's name reads twice at the call site: `WorkerSpec` in package `worker` is `worker.WorkerSpec`, and `worker.Spec` says the same. The message suggests the rest of the name. Words are compared without case, and the package name may span several words: `HTTPUtilProxy` in package `httputil` stutters too.

A name that is only the package name, as `time.Time`, is kept. So are methods, struct fields, unexported names, and every name in package `main` or in an external test package (`worker_test`).
