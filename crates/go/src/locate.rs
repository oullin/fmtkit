//! Finding executables: the helper beside fmtkit, and `go` on `PATH`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::{GoError, HELPER_ENV};

/// The helper's file name in every archive and install.
pub const HELPER_NAME: &str = "fmtkit-go-helper";

/// Locate the helper: `explicit`, then `FMTKIT_GO_HELPER`, then the install
/// locations around the running executable (see [`install_candidates`]), then
/// `PATH`. An explicit or environment path that is not an executable is an
/// error rather than a reason to keep looking.
pub fn helper(explicit: Option<&Path>) -> Result<PathBuf, GoError> {
    let exe = std::env::current_exe().ok();
    let canonical = exe.as_deref().and_then(|exe| exe.canonicalize().ok());

    resolve(explicit, std::env::var_os(HELPER_ENV), exe.as_deref(), canonical.as_deref(), std::env::var_os("PATH"), is_executable)
}

/// [`helper`] with every input supplied, so the order can be tested.
fn resolve(
    explicit: Option<&Path>,
    env: Option<OsString>,
    exe: Option<&Path>,
    canonical_exe: Option<&Path>,
    path_var: Option<OsString>,
    executable: impl Fn(&Path) -> bool,
) -> Result<PathBuf, GoError> {
    let named = explicit.map(Path::to_path_buf).or_else(|| env.filter(|value| !value.is_empty()).map(PathBuf::from));

    if let Some(path) = named {
        return if executable(&path) { Ok(path) } else { Err(GoError::NotFound(path)) };
    }

    let candidates = install_candidates(exe, canonical_exe);

    if let Some(found) = candidates.iter().find(|candidate| executable(candidate)) {
        return Ok(found.clone());
    }

    if let Some(found) = search_path(HELPER_NAME, path_var, &executable) {
        return Ok(found);
    }

    Err(GoError::NotFound(candidates.into_iter().next().unwrap_or_else(|| PathBuf::from(HELPER_NAME))))
}

/// Where an installed helper sits relative to the running `fmtkit`, in lookup
/// order:
///
/// 1. beside the executable as invoked (archives, `cargo install`);
/// 2. beside the executable with symlinks resolved;
/// 3. `../share/fmtkit/` from the resolved executable, where Homebrew puts the
///    extra archive files (`bin/fmtkit` links into `Cellar/fmtkit/<v>/bin`,
///    and the same Cellar directory holds `share/fmtkit`).
pub fn install_candidates(exe: Option<&Path>, canonical_exe: Option<&Path>) -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    let raw_dir = exe.and_then(Path::parent);
    let canonical_dir = canonical_exe.and_then(Path::parent);

    for dir in [raw_dir, canonical_dir].into_iter().flatten() {
        candidates.push(dir.join(HELPER_NAME));
    }

    if let Some(dir) = canonical_dir {
        candidates.push(dir.join("..").join("share").join("fmtkit").join(HELPER_NAME));
    }

    candidates.dedup();

    candidates
}

/// Find `name` on `PATH`.
pub fn which(name: &str) -> Option<PathBuf> {
    search_path(name, std::env::var_os("PATH"), &is_executable)
}

fn search_path(name: &str, path_var: Option<OsString>, executable: &impl Fn(&Path) -> bool) -> Option<PathBuf> {
    let path_var = path_var?;

    std::env::split_paths(&path_var).filter(|dir| !dir.as_os_str().is_empty()).map(|dir| dir.join(name)).find(|candidate| executable(candidate))
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

    use super::{HELPER_NAME, install_candidates, resolve};
    use crate::GoError;

    const BREW_LINK: &str = "/opt/homebrew/bin/fmtkit";
    const BREW_CELLAR: &str = "/opt/homebrew/Cellar/fmtkit/2.0.0/bin/fmtkit";

    fn path(text: &str) -> PathBuf {
        PathBuf::from(text)
    }

    #[test]
    fn lists_install_candidates_in_order() {
        let candidates = install_candidates(Some(Path::new(BREW_LINK)), Some(Path::new(BREW_CELLAR)));

        assert_eq!(
            candidates,
            [
                path("/opt/homebrew/bin/fmtkit-go-helper"),
                path("/opt/homebrew/Cellar/fmtkit/2.0.0/bin/fmtkit-go-helper"),
                path("/opt/homebrew/Cellar/fmtkit/2.0.0/bin/../share/fmtkit/fmtkit-go-helper"),
            ]
        );
    }

    #[test]
    fn collapses_candidates_when_the_executable_is_not_a_link() {
        let exe = Path::new("/usr/local/fmtkit/fmtkit");
        let candidates = install_candidates(Some(exe), Some(exe));

        assert_eq!(candidates, [path("/usr/local/fmtkit/fmtkit-go-helper"), path("/usr/local/fmtkit/../share/fmtkit/fmtkit-go-helper")]);
        assert_eq!(install_candidates(None, None), Vec::<PathBuf>::new());
    }

    #[test]
    fn prefers_explicit_then_env_then_install_then_path() {
        let everything = |_: &Path| true;
        let exe = Some(Path::new(BREW_LINK));
        let canonical = Some(Path::new(BREW_CELLAR));
        let path_var = || Some(OsString::from("/elsewhere/bin"));

        let found = resolve(Some(Path::new("/x/explicit")), Some("/x/env".into()), exe, canonical, path_var(), everything).unwrap();

        assert_eq!(found, path("/x/explicit"));

        let found = resolve(None, Some("/x/env".into()), exe, canonical, path_var(), everything).unwrap();

        assert_eq!(found, path("/x/env"));

        let found = resolve(None, Some(OsString::new()), exe, canonical, path_var(), everything).unwrap();

        assert_eq!(found, path("/opt/homebrew/bin/fmtkit-go-helper"));

        let only_share = |p: &Path| p.to_string_lossy().contains("share");
        let found = resolve(None, None, exe, canonical, path_var(), only_share).unwrap();

        assert_eq!(found, path("/opt/homebrew/Cellar/fmtkit/2.0.0/bin/../share/fmtkit/fmtkit-go-helper"));

        let only_path = |p: &Path| p.starts_with("/elsewhere");
        let found = resolve(None, None, exe, canonical, path_var(), only_path).unwrap();

        assert_eq!(found, path("/elsewhere/bin").join(HELPER_NAME));
    }

    #[test]
    fn a_named_helper_that_is_missing_is_an_error() {
        let nothing = |_: &Path| false;
        let err = resolve(Some(Path::new("/x/explicit")), None, None, None, None, nothing).unwrap_err();

        assert!(matches!(err, GoError::NotFound(p) if p == path("/x/explicit")));

        let err = resolve(None, Some("/x/env".into()), None, None, None, nothing).unwrap_err();

        assert!(matches!(err, GoError::NotFound(p) if p == path("/x/env")));
    }

    #[test]
    fn reports_the_first_install_location_when_nothing_is_found() {
        let nothing = |_: &Path| false;
        let err = resolve(None, None, Some(Path::new(BREW_LINK)), None, Some("/a:/b".into()), nothing).unwrap_err();

        assert!(matches!(err, GoError::NotFound(p) if p == path("/opt/homebrew/bin/fmtkit-go-helper")));

        let err = resolve(None, None, None, None, None, nothing).unwrap_err();

        assert!(matches!(err, GoError::NotFound(p) if p == path(HELPER_NAME)));
    }
}
