#![allow(dead_code)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use fmtkit_core::{Lang, is_declaration};
use fmtkit_discover::{Discovery, Scope, discover};

/// A scratch directory, optionally a git repository driven by the `git` CLI.
pub struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    pub fn plain() -> Self {
        Self { dir: tempfile::tempdir().unwrap() }
    }

    pub fn git() -> Self {
        let fixture = Self::plain();

        fixture.run(&["init", "-q", "-b", "main"]);

        fixture
    }

    /// A linked worktree of this repository, checked out in its own directory.
    pub fn linked_worktree(&self) -> Self {
        let linked = Self::plain();

        self.run(&["worktree", "add", "-q", "--detach", &linked.root().to_string_lossy()]);

        linked
    }

    pub fn root(&self) -> &Path {
        self.dir.path()
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.root().join(rel)
    }

    pub fn write(&self, rel: &str, text: &str) -> &Self {
        let path = self.path(rel);

        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();

        self
    }

    pub fn remove(&self, rel: &str) -> &Self {
        fs::remove_file(self.path(rel)).unwrap();

        self
    }

    /// Run git in the fixture with no user or system configuration.
    pub fn run(&self, args: &[&str]) -> Vec<u8> {
        let output = Command::new("git")
            .args(["-c", "user.name=fmtkit", "-c", "user.email=fmtkit@example.com", "-c", "commit.gpgsign=false", "-c", "core.autocrlf=false"])
            .args(args)
            .current_dir(self.root())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .output()
            .unwrap();

        assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));

        output.stdout
    }

    pub fn commit(&self, message: &str) -> &Self {
        self.run(&["add", "-A"]);
        self.run(&["commit", "-q", "-m", message]);

        self
    }

    pub fn discover(&self, scope: &Scope) -> Discovery {
        discover(self.root(), scope, &fmtkit_config::Files::default()).unwrap()
    }

    pub fn rels(&self, scope: &Scope) -> Vec<String> {
        self.discover(scope).files.into_iter().map(|file| file.rel).collect()
    }

    /// The changed set as v1 computed it with the git CLI, narrowed to files
    /// that still exist and that fmtkit owns.
    pub fn git_changed(&self) -> Vec<String> {
        let mut paths = self.list(&["ls-files", "--others", "--modified", "--exclude-standard", "-z"]);

        paths.extend(self.list(&["diff", "--cached", "--name-only", "--relative", "--diff-filter=d", "-z"]));

        self.owned(paths)
    }

    /// The `--all` set as v1 computed it with the git CLI, narrowed the same way.
    pub fn git_all(&self) -> Vec<String> {
        let paths = self.list(&["ls-files", "--cached", "--others", "--exclude-standard", "-z"]);

        self.owned(paths)
    }

    fn list(&self, args: &[&str]) -> BTreeSet<String> {
        self.run(args).split(|&b| b == 0).filter(|part| !part.is_empty()).map(|part| String::from_utf8(part.to_vec()).unwrap()).collect()
    }

    fn owned(&self, paths: BTreeSet<String>) -> Vec<String> {
        paths
            .into_iter()
            .filter(|rel| {
                let path = Path::new(rel);
                let on_disk = fs::symlink_metadata(self.path(rel)).is_ok_and(|meta| meta.is_file());
                let skipped_dir = rel.split('/').any(|name| matches!(name, "vendor" | "node_modules"));

                on_disk && !skipped_dir && Lang::from_path(path).is_some() && !is_declaration(path)
            })
            .collect()
    }
}

pub fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}
