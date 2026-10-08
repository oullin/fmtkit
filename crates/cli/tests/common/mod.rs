//! The fixture the end-to-end suites share: a scratch git repository, with
//! the cache and the Go helper pointed at per-test locations.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

pub const BIN: &str = env!("CARGO_BIN_EXE_fmtkit");

pub struct Fixture {
    pub dir: tempfile::TempDir,
    pub cache: tempfile::TempDir,
}

impl Fixture {
    pub fn new(files: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();

        git(dir.path(), &["init", "--quiet", "."]);

        let fixture = Self { dir, cache };

        for (path, content) in files {
            fixture.write(path, content);
        }

        fixture
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn write(&self, path: &str, content: &str) {
        let full = self.path().join(path);

        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, content).unwrap();
    }

    pub fn read(&self, path: &str) -> String {
        fs::read_to_string(self.path().join(path)).unwrap()
    }

    pub fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }

    /// fmtkit with `args`, in the repository, with the fixture's environment.
    pub fn command(&self, args: &[&str]) -> Command {
        self.command_with(Path::new(BIN), args)
    }

    /// [`Fixture::command`], running the fmtkit at `program`.
    pub fn command_with(&self, program: &Path, args: &[&str]) -> Command {
        let mut command = Command::new(program);

        command.args(args).current_dir(self.path()).env("FMTKIT_CACHE_DIR", self.cache.path()).env_remove("FMTKIT_CONFIG").env_remove("FMTKIT_JOBS");

        if let Some(helper) = go_helper() {
            command.env("FMTKIT_GO_HELPER", helper);
        }

        command
    }

    pub fn snapshot(&self) -> Vec<(PathBuf, Vec<u8>)> {
        let mut files: Vec<_> = walk(self.path()).into_iter().map(|p| (p.clone(), fs::read(&p).unwrap())).collect();

        files.sort();

        files
    }
}

pub fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();

    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();

        if path.file_name().is_some_and(|n| n == ".git") {
            continue;
        }

        if path.is_dir() {
            out.extend(walk(&path));
        } else {
            out.push(path);
        }
    }

    out
}

pub fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git").args(args).current_dir(dir).status().unwrap();

    assert!(status.success(), "git {args:?} failed");
}

/// The Go helper, built once per test binary, or `None` when Go is not installed.
pub fn go_helper() -> Option<&'static Path> {
    static HELPER: OnceLock<Option<PathBuf>> = OnceLock::new();

    HELPER
        .get_or_init(|| {
            let module = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../go/helper");
            let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fmtkit-go-helper");
            let built = Command::new("go").arg("build").arg("-o").arg(&out).arg(".").current_dir(module).status().ok()?;

            built.success().then_some(out)
        })
        .as_deref()
}

pub fn text(output: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr))
}

pub fn assert_exit(output: &Output, code: i32) {
    assert_eq!(output.status.code(), Some(code), "unexpected exit status; output:\n{}", text(output));
}
