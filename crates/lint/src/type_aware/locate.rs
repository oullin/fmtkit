//! Finding the native `tsrs` executable.

use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};

use super::TypeAwareError;

/// Names the `tsrs` executable to use, ahead of every other location.
pub const TSRS_ENV: &str = "FMTKIT_TSRS";

/// The executable's file name.
const NAME: &str = if cfg!(windows) { "tsrs.exe" } else { "tsrs" };

/// Where a lookup may look, gathered so the order can be tested.
struct Inputs<'a> {
    env: Option<OsString>,
    /// `[lint] tsrs`, already joined to the root.
    configured: Option<PathBuf>,
    exe: Option<&'a Path>,
    canonical_exe: Option<&'a Path>,
    root: &'a Path,
    path_var: Option<OsString>,
}

/// Locate `tsrs`, in this order:
///
/// 1. `FMTKIT_TSRS`;
/// 2. `[lint] tsrs`, relative to the repository root;
/// 3. beside the running fmtkit, as invoked, then with symlinks resolved;
/// 4. `node_modules/@tsrs/<platform>/tsrs` in `root` or an ancestor, where
///    `npm i -D tsrs` puts the native binary;
/// 5. `PATH`, skipping scripts such as npm's `node_modules/.bin/tsrs`, which is
///    a Node.js launcher.
///
/// A path named by the environment or the configuration that is not an
/// executable is an error rather than a reason to keep looking.
///
/// The path returned is absolute: tsrs runs from `root`, where a relative
/// path, such as `FMTKIT_TSRS=./tools/tsrs` or a relative `PATH` entry, would
/// name another file. A relative path that cannot be made absolute, because
/// the current directory is gone, is an error rather than a guess.
pub fn locate(root: &Path, configured: Option<&str>) -> Result<PathBuf, TypeAwareError> {
    let exe = std::env::current_exe().ok();
    let canonical = exe.as_deref().and_then(|exe| exe.canonicalize().ok());
    let inputs = Inputs {
        env: std::env::var_os(TSRS_ENV),
        configured: configured.map(|path| root.join(path)),
        exe: exe.as_deref(),
        canonical_exe: canonical.as_deref(),
        root,
        path_var: std::env::var_os("PATH"),
    };

    let found = resolve(inputs, is_executable, is_script)?;

    // Only a relative path needs the current directory, which may be gone.
    std::path::absolute(&found).map_err(|source| TypeAwareError::Spawn { path: found, source })
}

fn resolve(inputs: Inputs<'_>, executable: impl Fn(&Path) -> bool, script: impl Fn(&Path) -> bool) -> Result<PathBuf, TypeAwareError> {
    let named = [(inputs.env.filter(|value| !value.is_empty()).map(PathBuf::from), TSRS_ENV), (inputs.configured, "[lint] tsrs")];

    for (path, named_by) in named {
        if let Some(path) = path {
            return if executable(&path) { Ok(path) } else { Err(TypeAwareError::NotExecutable { path, named_by }) };
        }
    }

    let beside = [inputs.exe, inputs.canonical_exe].into_iter().flatten().filter_map(Path::parent).map(|dir| dir.join(NAME));
    let npm =
        inputs.root.ancestors().flat_map(|dir| platform_packages().into_iter().map(move |package| dir.join("node_modules/@tsrs").join(package).join(NAME)));

    if let Some(found) = beside.chain(npm).find(|candidate| executable(candidate)) {
        return Ok(found);
    }

    let path_var = inputs.path_var.unwrap_or_default();
    let on_path = std::env::split_paths(&path_var).filter(|dir| !dir.as_os_str().is_empty()).map(|dir| dir.join(NAME));

    on_path.into_iter().find(|candidate| executable(candidate) && !script(candidate)).ok_or(TypeAwareError::NotFound { rules: Vec::new() })
}

/// The `@tsrs/*` packages that may hold the native binary for this platform,
/// preferred first.
fn platform_packages() -> Vec<String> {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        other => other,
    };
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "x64",
        other => other,
    };

    if os != "linux" {
        return vec![format!("{os}-{arch}")];
    }

    let libcs = if cfg!(target_env = "musl") { ["musl", "gnu"] } else { ["gnu", "musl"] };

    libcs.iter().map(|libc| format!("linux-{arch}-{libc}")).collect()
}

/// A file that starts with `#!`: an interpreter script, not the native binary.
fn is_script(path: &Path) -> bool {
    let mut start = [0; 2];

    std::fs::File::open(path).and_then(|mut file| file.read_exact(&mut start)).is_ok_and(|()| start == *b"#!")
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    path.metadata().is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    use super::{Inputs, NAME, TSRS_ENV, platform_packages, resolve};
    use crate::type_aware::TypeAwareError;

    fn inputs(env: Option<&str>, configured: Option<&str>) -> Inputs<'static> {
        Inputs {
            env: env.map(OsString::from),
            configured: configured.map(PathBuf::from),
            exe: Some(Path::new("/opt/bin/fmtkit")),
            canonical_exe: Some(Path::new("/opt/Cellar/fmtkit/bin/fmtkit")),
            root: Path::new("/work/repo/app"),
            path_var: Some(OsString::from("/work/repo/node_modules/.bin:/usr/local/bin")),
        }
    }

    fn npm(dir: &str) -> PathBuf {
        Path::new(dir).join("node_modules/@tsrs").join(&platform_packages()[0]).join(NAME)
    }

    fn not_script(_: &Path) -> bool {
        false
    }

    #[test]
    fn prefers_the_environment_then_the_configuration() {
        let everything = |_: &Path| true;

        assert_eq!(resolve(inputs(Some("/x/env"), Some("/x/config")), everything, not_script).unwrap(), Path::new("/x/env"));
        assert_eq!(resolve(inputs(None, Some("/x/config")), everything, not_script).unwrap(), Path::new("/x/config"));
        assert_eq!(resolve(inputs(Some(""), Some("/x/config")), everything, not_script).unwrap(), Path::new("/x/config"));
    }

    #[test]
    fn a_named_path_that_is_not_executable_is_an_error() {
        let nothing = |_: &Path| false;

        let error = resolve(inputs(Some("/x/env"), Some("/x/config")), nothing, not_script).unwrap_err();

        assert!(matches!(&error, TypeAwareError::NotExecutable { path, named_by } if path == Path::new("/x/env") && *named_by == TSRS_ENV), "{error}");

        let error = resolve(inputs(None, Some("/x/config")), nothing, not_script).unwrap_err();

        assert!(matches!(&error, TypeAwareError::NotExecutable { named_by, .. } if *named_by == "[lint] tsrs"), "{error}");
    }

    #[test]
    fn then_beside_fmtkit_then_node_modules_then_path() {
        let only = |wanted: PathBuf| move |candidate: &Path| candidate == wanted;

        let beside = Path::new("/opt/bin").join(NAME);
        let resolved = Path::new("/opt/Cellar/fmtkit/bin").join(NAME);
        let ancestor = npm("/work/repo");
        let on_path = Path::new("/usr/local/bin").join(NAME);

        for wanted in [&beside, &resolved, &npm("/work/repo/app"), &ancestor, &on_path] {
            assert_eq!(&resolve(inputs(None, None), only(wanted.clone()), not_script).unwrap(), wanted);
        }

        let everything = |_: &Path| true;

        assert_eq!(resolve(inputs(None, None), everything, not_script).unwrap(), beside);

        let not_beside = |candidate: &Path| !candidate.starts_with("/opt");

        assert_eq!(resolve(inputs(None, None), not_beside, not_script).unwrap(), npm("/work/repo/app"));
    }

    /// npm's `node_modules/.bin/tsrs` is a Node.js script; only the native
    /// binary will do.
    #[test]
    fn skips_scripts_on_path() {
        let on_path = |candidate: &Path| candidate.parent().is_some_and(|dir| dir.ends_with("node_modules/.bin") || dir == Path::new("/usr/local/bin"));
        let launcher = |candidate: &Path| candidate.starts_with("/work/repo/node_modules/.bin");

        assert_eq!(resolve(inputs(None, None), on_path, launcher).unwrap(), Path::new("/usr/local/bin").join(NAME));

        let only_launcher = |candidate: &Path| candidate.starts_with("/work/repo/node_modules/.bin");

        assert!(matches!(resolve(inputs(None, None), only_launcher, launcher), Err(TypeAwareError::NotFound { .. })));
    }

    #[test]
    fn nothing_found_is_not_found() {
        let nothing = |_: &Path| false;
        let error = resolve(Inputs { path_var: None, ..inputs(None, None) }, nothing, not_script).unwrap_err();

        assert!(matches!(error, TypeAwareError::NotFound { .. }));
    }

    #[test]
    fn names_the_npm_package_for_this_platform() {
        let packages = platform_packages();

        assert_ne!(packages.len(), 0);

        if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            assert_eq!(packages, ["darwin-arm64"]);
        }

        if cfg!(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu")) {
            assert_eq!(packages, ["linux-x64-gnu", "linux-x64-musl"]);
        }
    }
}
