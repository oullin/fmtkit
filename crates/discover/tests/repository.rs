//! A repository kept open from one run to the next.

mod common;

use std::time::{Duration, SystemTime};

use common::{Fixture, strings};
use fmtkit_discover::{Repository, Scope, discover_with};

/// Discover through `repository`, as a run a minute from now would, so
/// everything just written counts as settled and the repository is kept.
fn kept(repo: &Fixture, repository: &mut Repository, scope: &Scope) -> Vec<String> {
    repository.next_run(SystemTime::now() + Duration::from_secs(60));

    let found = discover_with(repo.root(), scope, &fmtkit_config::Files::default(), None, repository).unwrap();

    found.files.into_iter().map(|file| file.rel).collect()
}

fn assert_matches_git(repo: &Fixture, repository: &mut Repository, step: &str) {
    let all = Scope { all: true, ..Scope::default() };

    assert_eq!(kept(repo, repository, &Scope::default()), repo.git_changed(), "changed set after {step}");
    assert_eq!(kept(repo, repository, &all), repo.git_all(), "all files after {step}");
}

fn repo() -> Fixture {
    let repo = Fixture::git();

    repo.write("a.ts", "export const a = 1;\n").write("src/b.ts", "export const b = 1;\n").commit("init");

    repo
}

#[test]
fn a_kept_repository_follows_the_index_head_and_worktree() {
    let repo = repo();
    let mut repository = Repository::default();

    assert_matches_git(&repo, &mut repository, "the first run");

    repo.write("src/b.ts", "export const b = 2;\n").write("staged.ts", "export {};\n");
    repo.run(&["add", "src/b.ts", "staged.ts"]);
    assert_matches_git(&repo, &mut repository, "staging");

    repo.commit("next");
    assert_matches_git(&repo, &mut repository, "a commit");

    repo.write("new.ts", "export {};\n").remove("a.ts");
    assert_matches_git(&repo, &mut repository, "worktree changes");

    repo.run(&["checkout", "-q", "-b", "other"]);
    repo.commit("other");
    repo.run(&["checkout", "-q", "main"]);
    assert_matches_git(&repo, &mut repository, "switching branches");
}

#[cfg(unix)]
#[test]
fn a_kept_repository_is_reopened_when_its_configuration_changes() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let repo = repo();
    let mut repository = Repository::default();

    fs::set_permissions(repo.path("a.ts"), fs::Permissions::from_mode(0o755)).unwrap();

    assert_eq!(repo.git_changed(), strings(&["a.ts"]));
    assert_eq!(kept(&repo, &mut repository, &Scope::default()), strings(&["a.ts"]));

    // Only the configuration changes: git no longer compares the executable bit.
    repo.run(&["config", "core.fileMode", "false"]);

    assert_eq!(repo.git_changed(), Vec::<String>::new());
    assert_eq!(kept(&repo, &mut repository, &Scope::default()), Vec::<String>::new());
}

#[test]
fn a_kept_repository_serves_one_root_at_a_time() {
    let first = repo();
    let second = Fixture::git();
    let mut repository = Repository::default();

    second.write("other.ts", "export {};\n");

    assert_eq!(kept(&first, &mut repository, &Scope::default()), Vec::<String>::new());
    assert_eq!(kept(&second, &mut repository, &Scope::default()), strings(&["other.ts"]));
    assert_eq!(kept(&first, &mut repository, &Scope::default()), Vec::<String>::new());
}
