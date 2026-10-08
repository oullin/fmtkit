//! A watched repository: each run looks only at what changed since the one
//! before, and finds what git finds.

#![cfg(any(target_os = "macos", target_os = "linux"))]

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, SystemTime};

use common::{Fixture, strings};
use fmtkit_discover::{Memory, Repository, Scope, discover_with};

/// A watching repository and the memory beside it, as a server keeps them.
struct Watched {
    repository: Repository,
    memory: Memory,
    _store: tempfile::TempDir,
}

impl Watched {
    fn new() -> Self {
        let store = tempfile::tempdir().unwrap();
        let memory = Memory::open(store.path().join("dirs"));
        let mut repository = Repository::default();

        repository.watch();

        Self { repository, memory, _store: store }
    }

    /// Discover as a run a minute from now would, so everything just written
    /// counts as settled.
    fn rels(&mut self, repo: &Fixture, scope: &Scope) -> Vec<String> {
        let later = SystemTime::now() + Duration::from_secs(60);

        self.memory.next_run(later);
        self.repository.next_run(later);

        let found = discover_with(repo.root(), scope, &fmtkit_config::Files::default(), Some(&self.memory), &mut self.repository).unwrap();

        self.memory.save().unwrap();

        assert_eq!(self.repository.watch_failure(), None);

        found.files.into_iter().map(|file| file.rel).collect()
    }

    /// Both scopes, twice, against the git CLI.
    fn assert_matches_git(&mut self, repo: &Fixture, step: &str) {
        let all = Scope { all: true, ..Scope::default() };

        for _ in 0..2 {
            assert_eq!(self.rels(repo, &Scope::default()), repo.git_changed(), "changed set after {step}");
            assert_eq!(self.rels(repo, &all), repo.git_all(), "all files after {step}");
        }
    }
}

/// Every file backdated before the commit, so stat alone vouches for each
/// entry, and only the watch can tell a later change.
fn settled_repo(files: &[(&str, &str)]) -> Fixture {
    let repo = Fixture::git();

    for (rel, text) in files {
        repo.write(rel, text);
        backdate(&repo, rel);
    }

    repo.commit("init");
    repo
}

fn backdate(repo: &Fixture, rel: &str) {
    let file = fs::File::options().write(true).open(repo.path(rel)).unwrap();

    file.set_modified(SystemTime::now() - Duration::from_secs(3600)).unwrap();
}

#[test]
fn a_watched_repository_follows_every_change_to_the_tree() {
    let repo = settled_repo(&[
        (".gitignore", "dist/\n"),
        ("a.ts", "export const a = 1;\n"),
        ("src/b.ts", "export const b = 1;\n"),
        ("src/deep/c.ts", "export const c = 1;\n"),
        ("lib/d.go", "package lib\n"),
    ]);
    let mut watched = Watched::new();

    watched.assert_matches_git(&repo, "the first run");
    assert_eq!(watched.rels(&repo, &Scope::default()), Vec::<String>::new());

    // The same size, so only the times tell.
    repo.write("src/b.ts", "export const b = 2;\n");
    watched.assert_matches_git(&repo, "an edit in place");

    // Written beside, then renamed over the file, as editors save.
    repo.write("src/deep/.c.ts.tmp", "export const c = 2;\n");
    fs::rename(repo.path("src/deep/.c.ts.tmp"), repo.path("src/deep/c.ts")).unwrap();
    watched.assert_matches_git(&repo, "an atomic replace");

    fs::set_permissions(repo.path("a.ts"), fs::Permissions::from_mode(0o755)).unwrap();
    watched.assert_matches_git(&repo, "a mode change");

    repo.remove("lib/d.go").write("lib/d.go", "package lib // again\n");
    watched.assert_matches_git(&repo, "a removal and a new file in its place");

    repo.write("new.ts", "export {};\n").write("fresh/e.ts", "export {};\n").write("dist/out.ts", "export {};\n");
    watched.assert_matches_git(&repo, "new files and directories");

    fs::rename(repo.path("src/deep"), repo.path("src/deeper")).unwrap();
    watched.assert_matches_git(&repo, "a directory renamed");

    fs::rename(repo.path("src/deeper"), repo.path("src/deep")).unwrap();
    watched.assert_matches_git(&repo, "a directory renamed back");

    fs::remove_dir_all(repo.path("fresh")).unwrap();
    watched.assert_matches_git(&repo, "a directory removed");

    repo.write(".gitignore", "dist/\nnew.ts\n");
    watched.assert_matches_git(&repo, "an edited .gitignore");

    repo.write("lib/extra.go", "package lib\n");
    watched.assert_matches_git(&repo, "a new file below");

    repo.write("lib/.gitignore", "extra.go\n");
    watched.assert_matches_git(&repo, "a new nested .gitignore");

    repo.run(&["add", "-A"]);
    watched.assert_matches_git(&repo, "staging");

    repo.run(&["commit", "-q", "-m", "next"]);
    watched.assert_matches_git(&repo, "a commit");
    assert_eq!(watched.rels(&repo, &Scope::default()), Vec::<String>::new());
}

#[test]
fn a_run_over_every_file_leaves_what_was_reported_to_the_next() {
    let repo = settled_repo(&[("a.ts", "export const a = 1;\n"), ("src/b.ts", "export const b = 1;\n")]);
    let all = Scope { all: true, ..Scope::default() };
    let mut watched = Watched::new();

    assert_eq!(watched.rels(&repo, &Scope::default()), Vec::<String>::new());

    repo.write("src/b.ts", "export const b = 2;\n");

    assert_eq!(watched.rels(&repo, &all), strings(&["a.ts", "src/b.ts"]));
    assert_eq!(watched.rels(&repo, &Scope::default()), strings(&["src/b.ts"]));
}

#[test]
fn a_flood_of_changes_starts_the_watch_over() {
    let names: Vec<String> = (0..1200).map(|n| format!("many/f{n}.ts")).collect();
    let files: Vec<(&str, &str)> = names.iter().map(|rel| (rel.as_str(), "export const x = 1;\n")).collect();
    let repo = settled_repo(&files);
    let mut watched = Watched::new();

    watched.assert_matches_git(&repo, "the first run");

    for rel in &names {
        repo.write(rel, "export const x = 2;\n");
    }

    watched.assert_matches_git(&repo, "editing every file");
    assert_eq!(watched.rels(&repo, &Scope::default()).len(), names.len());
}

#[test]
fn a_watch_follows_one_root_at_a_time() {
    let first = settled_repo(&[("a.ts", "export const a = 1;\n")]);
    let second = settled_repo(&[("b.ts", "export const b = 1;\n")]);
    let mut watched = Watched::new();

    assert_eq!(watched.rels(&first, &Scope::default()), Vec::<String>::new());
    assert_eq!(watched.rels(&second, &Scope::default()), Vec::<String>::new());

    first.write("a.ts", "export const a = 2;\n");
    assert_eq!(watched.rels(&first, &Scope::default()), strings(&["a.ts"]));
}
