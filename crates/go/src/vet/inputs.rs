//! What one `go vet` run depends on, digested so a run that passed can be
//! skipped while nothing it read has changed.
//!
//! The digest covers the `go` executable, the environment `go` reads (every
//! `GO*` and `CGO_*` variable, the C toolchain variables, `HOME`, and the
//! `go env -w` file), the packages named, and the stamp (inode, size,
//! modification and change times) of every file in the module, ignored or
//! not, outside the directories `go` skips and the modules nested inside it.
//! Dependencies from the module cache are pinned by `go.sum`. A module whose
//! code can come from elsewhere on disk, through a `go.work` workspace or a
//! `replace` with a local path, has no digest and is always vetted.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use ignore::{WalkBuilder, WalkState};

use super::{ModuleRun, skipped_dir};

/// How much older than the run every file must be before a pass is
/// remembered; coarse file systems keep two-second timestamps. The outcome
/// cache follows the same rule.
const SETTLE: Duration = Duration::from_secs(2);

/// Variables `go` reads besides `GO*` and `CGO_*`.
const TOOLCHAIN_VARS: [&str; 6] = ["AR", "CC", "CXX", "FC", "HOME", "PKG_CONFIG"];

pub(super) struct Inputs {
    pub key: [u8; 32],
    /// Every file is older than [`SETTLE`], so the key can be trusted later.
    pub settled: bool,
}

/// The digest of what vetting `module` with `go` reads, or `None` when it
/// cannot be known from the module alone.
pub(super) fn fingerprint(go: &Path, module: &ModuleRun) -> Option<Inputs> {
    let go_mod = fs::read_to_string(module.dir.join("go.mod")).ok()?;

    if in_workspace(module) || replaces_locally(&go_mod) {
        return None;
    }

    let files = stamps(&module.dir)?;
    let settled = SystemTime::now().checked_sub(SETTLE).map_or(0, nanos_since_epoch);
    let mut hasher = blake3::Hasher::new();

    field(&mut hasher, b"go vet");
    field(&mut hasher, fmtkit_core::VERSION.as_bytes());
    field(&mut hasher, go.canonicalize().unwrap_or_else(|_| go.to_path_buf()).as_os_str().as_encoded_bytes());
    stamp(&mut hasher, fs::metadata(go).ok().as_ref());
    stamp(&mut hasher, env_file().and_then(|path| fs::metadata(path).ok()).as_ref());

    for (name, value) in environment() {
        field(&mut hasher, name.as_encoded_bytes());
        field(&mut hasher, value.as_encoded_bytes());
    }

    field(&mut hasher, module.dir.as_os_str().as_encoded_bytes());
    hasher.update(&[u8::from(module.outside_workspace)]);

    for package in &module.packages {
        field(&mut hasher, package.as_bytes());
    }

    for (rel, meta) in &files {
        field(&mut hasher, rel.as_os_str().as_encoded_bytes());
        stamp(&mut hasher, Some(meta));
    }

    let newest = files.iter().map(|(_, meta)| modified(meta)).max().unwrap_or(0);

    Some(Inputs { key: *hasher.finalize().as_bytes(), settled: newest < settled })
}

/// Whether `go` builds `module` in workspace mode, where other modules on
/// disk replace its dependencies.
fn in_workspace(module: &ModuleRun) -> bool {
    match std::env::var_os("GOWORK") {
        Some(value) if value == "off" => false,
        Some(value) if !value.is_empty() => true,
        _ => !module.outside_workspace && module.dir.ancestors().any(|dir| dir.join("go.work").is_file()),
    }
}

/// Whether a `go.mod` replaces a dependency with a directory on disk.
fn replaces_locally(go_mod: &str) -> bool {
    let mut in_block = false;

    go_mod.lines().any(|line| {
        let line = line.split("//").next().unwrap_or("").trim();

        let spec = if in_block {
            in_block = !line.starts_with(')');

            line
        } else if let Some(rest) = line.strip_prefix("replace").filter(|rest| rest.starts_with([' ', '\t', '('])) {
            let rest = rest.trim();

            match rest.strip_prefix('(') {
                Some(inner) => {
                    in_block = !inner.trim_end().ends_with(')');

                    inner.trim_end_matches(')')
                }
                None => rest,
            }
        } else {
            return false;
        };

        spec.split_once("=>").and_then(|(_, target)| target.split_whitespace().next()).is_some_and(|target| is_local(&super::unquote(target)))
    })
}

/// `go`'s rule for a replacement path rather than a module path.
fn is_local(target: &str) -> bool {
    target == "." || target == ".." || target.starts_with("./") || target.starts_with("../") || Path::new(target).is_absolute() || target.starts_with('\\')
}

/// Every file `go` could read under `dir`, with its metadata, sorted. `None`
/// when the walk fails.
fn stamps(dir: &Path) -> Option<Vec<(PathBuf, fs::Metadata)>> {
    let found = Mutex::new(Vec::new());
    let failed = Mutex::new(false);
    let base = dir.to_path_buf();

    WalkBuilder::new(dir)
        .standard_filters(false)
        .follow_links(false)
        .filter_entry(move |entry| {
            if entry.depth() == 0 || !entry.file_type().is_some_and(|kind| kind.is_dir()) {
                return true;
            }

            let name = entry.file_name().to_string_lossy();

            !skipped_dir(&name) && !entry.path().join("go.mod").is_file() && entry.path().starts_with(&base)
        })
        .build_parallel()
        .run(|| {
            let failed = &failed;
            let mut sink = Drain { local: Vec::new(), found: &found };

            Box::new(move |entry| {
                let Ok(entry) = entry else {
                    *failed.lock().unwrap_or_else(PoisonError::into_inner) = true;

                    return WalkState::Quit;
                };

                if entry.file_type().is_some_and(|kind| !kind.is_dir())
                    && let Ok(meta) = fs::metadata(entry.path())
                    && let Ok(rel) = entry.path().strip_prefix(dir)
                {
                    sink.local.push((rel.to_path_buf(), meta));
                }

                WalkState::Continue
            })
        });

    if failed.into_inner().unwrap_or_else(PoisonError::into_inner) {
        return None;
    }

    let mut files = found.into_inner().unwrap_or_else(PoisonError::into_inner);

    files.sort_unstable_by(|a, b| a.0.cmp(&b.0));

    Some(files)
}

/// One walker thread's files, handed over when its visitor is dropped.
struct Drain<'a> {
    local: Vec<(PathBuf, fs::Metadata)>,
    found: &'a Mutex<Vec<(PathBuf, fs::Metadata)>>,
}

impl Drop for Drain<'_> {
    fn drop(&mut self) {
        self.found.lock().unwrap_or_else(PoisonError::into_inner).append(&mut self.local);
    }
}

/// The variables `go` reads, sorted by name.
fn environment() -> Vec<(OsString, OsString)> {
    let mut vars: Vec<(OsString, OsString)> = std::env::vars_os()
        .filter(|(name, _)| {
            let name = name.to_string_lossy();

            name.starts_with("GO") || name.starts_with("CGO_") || TOOLCHAIN_VARS.contains(&name.as_ref())
        })
        .collect();

    vars.sort();
    vars
}

/// The file `go env -w` writes, which sets defaults for every `go` command.
fn env_file() -> Option<PathBuf> {
    match std::env::var_os("GOENV") {
        Some(path) if path == "off" => None,
        Some(path) if !path.is_empty() => Some(PathBuf::from(path)),
        _ => dirs::config_dir().map(|dir| dir.join("go").join("env")),
    }
}

fn field(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hasher.update(&(bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

/// Hash what a file's metadata says about its content; an absent file
/// hashes differently from every present one.
fn stamp(hasher: &mut blake3::Hasher, meta: Option<&fs::Metadata>) {
    let Some(meta) = meta else {
        hasher.update(&[0]);

        return;
    };

    hasher.update(&[1]);
    hasher.update(&meta.len().to_le_bytes());
    hasher.update(&modified(meta).to_le_bytes());

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        for value in [meta.dev(), meta.ino(), meta.mode().into()] {
            hasher.update(&value.to_le_bytes());
        }

        hasher.update(&meta.ctime().saturating_mul(1_000_000_000).saturating_add(meta.ctime_nsec()).to_le_bytes());
    }
}

/// Nanoseconds since the Unix epoch.
fn modified(meta: &fs::Metadata) -> i64 {
    meta.modified().map_or(i64::MAX, nanos_since_epoch)
}

fn nanos_since_epoch(time: SystemTime) -> i64 {
    time.duration_since(SystemTime::UNIX_EPOCH).map_or(0, |since| i64::try_from(since.as_nanos()).unwrap_or(i64::MAX))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::time::{Duration, SystemTime};

    use super::{fingerprint, replaces_locally};
    use crate::vet::ModuleRun;

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    /// Age a file past the settle window, keeping its content.
    fn age(path: &Path) {
        let old = SystemTime::now() - Duration::from_secs(60);

        fs::File::options().write(true).open(path).unwrap().set_modified(old).unwrap();
    }

    fn module(dir: &Path) -> ModuleRun {
        ModuleRun { dir: dir.to_path_buf(), packages: vec!["./...".into()], outside_workspace: false }
    }

    fn key(dir: &Path) -> [u8; 32] {
        fingerprint(Path::new("/usr/bin/true"), &module(dir)).unwrap().key
    }

    #[test]
    fn follows_every_file_go_reads_and_nothing_else() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        write(&root.join("go.mod"), "module m\n\ngo 1.27\n");
        write(&root.join("a.go"), "package a\n");

        let first = key(root);

        assert_eq!(key(root), first);

        // Directories go skips and nested modules are not inputs.
        write(&root.join("testdata/x.go"), "package x\n");
        write(&root.join(".cache/x.go"), "package x\n");
        write(&root.join("_old/x.go"), "package x\n");
        write(&root.join("nested/go.mod"), "module nested\n");
        write(&root.join("nested/n.go"), "package n\n");

        assert_eq!(key(root), first);

        // Any other file is, ignored by git or not: an embedded asset, a
        // generated file, a new package.
        write(&root.join("assets/logo.txt"), "logo\n");

        let with_asset = key(root);

        assert_ne!(with_asset, first);

        write(&root.join("a.go"), "package a // edited\n");

        assert_ne!(key(root), with_asset);
    }

    #[test]
    fn trusts_only_settled_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        write(&root.join("go.mod"), "module m\n");
        write(&root.join("a.go"), "package a\n");

        assert!(!fingerprint(Path::new("/usr/bin/true"), &module(root)).unwrap().settled);

        age(&root.join("go.mod"));
        age(&root.join("a.go"));

        assert!(fingerprint(Path::new("/usr/bin/true"), &module(root)).unwrap().settled);
    }

    #[test]
    fn modules_that_read_other_directories_have_no_fingerprint() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        write(&root.join("local/go.mod"), "module m\n\nreplace example.com/x => ../x\n");
        write(&root.join("remote/go.mod"), "module m\n\nreplace example.com/x => example.com/y v1.0.0\n");
        write(&root.join("work/go.work"), "go 1.27\n\nuse ./m\n");
        write(&root.join("work/m/go.mod"), "module m\n");

        assert!(fingerprint(Path::new("/usr/bin/true"), &module(&root.join("local"))).is_none());
        assert!(fingerprint(Path::new("/usr/bin/true"), &module(&root.join("remote"))).is_some());
        assert!(fingerprint(Path::new("/usr/bin/true"), &module(&root.join("work/m"))).is_none());
        assert!(fingerprint(Path::new("/usr/bin/true"), &module(&root.join("missing"))).is_none());
    }

    #[test]
    fn finds_local_replacements() {
        assert!(replaces_locally("replace a => ./a\n"));
        assert!(replaces_locally("replace a v1.0.0 => ../a\n"));
        assert!(replaces_locally("replace a => /abs/a\n"));
        assert!(replaces_locally("replace (\n\ta => b v1.0.0\n\tc => \"../c\" // local\n)\n"));
        assert!(replaces_locally("replace (a => ..)\n"));
        assert!(!replaces_locally("replace a => b v1.0.0\n"));
        assert!(!replaces_locally("replace (\n\ta => b v1.0.0\n)\nrequire c v1.0.0\n"));
        assert!(!replaces_locally("// replace a => ./a\nrequire ./a v1.0.0\n"));
        assert!(!replaces_locally("replacements a => ./a\n"));
    }
}
