mod common;

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use common::{Fixture, strings};
use fmtkit_discover::{Memory, Repository, Scope, discover_with};

/// Discover with the memory in `store`, as a run a minute from now would, so
/// everything just written counts as settled and is stored.
fn remembered(repo: &Fixture, store: &Path, scope: &Scope) -> Vec<String> {
    let mut memory = Memory::open_as_of(store, SystemTime::now() + Duration::from_secs(60));
    let found = discover_with(repo.root(), scope, &fmtkit_config::Files::default(), Some(&memory), &mut Repository::default()).unwrap();

    memory.save().unwrap();

    found.files.into_iter().map(|file| file.rel).collect()
}

/// Both scopes, from a fresh walk and from the memory, against the git CLI.
fn assert_matches_git(repo: &Fixture, store: &Path, step: &str) {
    let all = Scope { all: true, ..Scope::default() };

    for _ in 0..2 {
        assert_eq!(remembered(repo, store, &Scope::default()), repo.git_changed(), "changed set after {step}");
        assert_eq!(remembered(repo, store, &all), repo.git_all(), "all files after {step}");
    }
}

fn repo() -> Fixture {
    let repo = Fixture::git();

    repo.write(".gitignore", "dist/\n")
        .write("a.ts", "export const a = 1;\n")
        .write("src/b.ts", "export const b = 1;\n")
        .write("src/deep/c.ts", "export const c = 1;\n")
        .write("lib/d.go", "package lib\n")
        .commit("init");

    repo
}

#[test]
fn replayed_listings_follow_every_change_to_the_tree() {
    let repo = repo();
    let store = repo.path(".git/fmtkit-test.dirs");

    assert_matches_git(&repo, &store, "the first walk");

    repo.write("src/new.ts", "export {};\n");
    assert_matches_git(&repo, &store, "an untracked file");

    repo.write("src/deep/er/est/e.ts", "export {};\n");
    assert_matches_git(&repo, &store, "a new nested directory");

    repo.write("src/b.ts", "export const b = 2;\n");
    assert_matches_git(&repo, &store, "a tracked edit");

    repo.remove("src/new.ts");
    assert_matches_git(&repo, &store, "a removed file");

    fs::rename(repo.path("src/deep/er"), repo.path("src/deep/moved")).unwrap();
    assert_matches_git(&repo, &store, "a renamed directory");

    repo.write("dist/out.ts", "export {};\n");
    assert_matches_git(&repo, &store, "a file in an ignored directory");

    repo.write("lib/sub/repo/x.ts", "export {};\n");
    repo.run(&["init", "-q", "lib/sub/repo"]);
    assert_matches_git(&repo, &store, "a nested repository");
}

#[test]
fn edited_ignore_files_re_list_what_they_cover() {
    let repo = repo();
    let store = repo.path(".git/fmtkit-test.dirs");

    repo.write("src/deep/skip.ts", "export {};\n").write("src/keep.ts", "export {};\n");
    assert_matches_git(&repo, &store, "the first walk");

    // Written in place: the directories holding the files keep their stamps.
    repo.write(".gitignore", "dist/\nskip.ts\n");
    assert_matches_git(&repo, &store, "a root pattern");

    repo.write("src/.gitignore", "!skip.ts\n");
    assert_matches_git(&repo, &store, "a deeper negation");

    repo.write("src/.gitignore", "keep.ts\n");
    assert_matches_git(&repo, &store, "a deeper pattern");

    repo.write(".git/info/exclude", "deep/\n");
    assert_matches_git(&repo, &store, "an exclude pattern");

    repo.remove(".git/info/exclude");
    repo.remove("src/.gitignore");
    assert_matches_git(&repo, &store, "removed ignore files");
}

#[test]
fn an_unreadable_store_starts_over() {
    let repo = repo();
    let store = repo.path(".git/fmtkit-test.dirs");

    repo.write("src/new.ts", "export {};\n");
    fs::write(&store, b"not a store").unwrap();
    assert_matches_git(&repo, &store, "a corrupt store");

    fs::write(&store, &fs::read(&store).unwrap()[..20]).unwrap();
    assert_matches_git(&repo, &store, "a truncated store");
}

#[test]
fn unsettled_directories_are_not_stored() {
    let repo = repo();
    let store = repo.path(".git/fmtkit-test.dirs");
    let mut memory = Memory::open(&store);

    discover_with(repo.root(), &Scope::default(), &fmtkit_config::Files::default(), Some(&memory), &mut Repository::default()).unwrap();
    memory.save().unwrap();

    // Everything was written within the settle window, so nothing was kept
    // to replay, and a later change is seen.
    repo.write("src/new.ts", "export {};\n");
    assert_eq!(remembered(&repo, &store, &Scope::default()), strings(&["src/new.ts"]));
}

/// Backdate every file, then commit, so the index entries are older than the
/// index and their stat data alone vouches for them.
fn settled_repo() -> Fixture {
    let repo = Fixture::git();

    repo.write("a.ts", "export const a = 1;\n").write("src/b.ts", "export const b = 1;\n").write("src/c.ts", "export const c = 1;\n");

    for rel in ["a.ts", "src/b.ts", "src/c.ts"] {
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
fn stat_vouches_only_for_untouched_entries() {
    let repo = settled_repo();

    assert_eq!(repo.rels(&Scope::default()), Vec::<String>::new());

    // The same size, so only the times tell.
    repo.write("src/b.ts", "export const b = 2;\n");
    assert_eq!(repo.rels(&Scope::default()), repo.git_changed());
    assert_eq!(repo.rels(&Scope::default()), strings(&["src/b.ts"]));

    repo.remove("a.ts");
    repo.run(&["add", "src/b.ts"]);
    repo.write("src/c.ts", "export const c = 22;\n");
    assert_eq!(repo.rels(&Scope::default()), repo.git_changed());
    assert_eq!(repo.rels(&Scope::default()), strings(&["src/b.ts", "src/c.ts"]));
}

#[test]
fn many_suspect_entries_fall_back_to_a_full_status() {
    let repo = Fixture::git();

    // Written in the same second as the index, so every entry is racy.
    for i in 0..300 {
        repo.write(&format!("f{i}.ts"), "export {};\n");
    }

    repo.commit("init");
    repo.write("f7.ts", "export const changed = 1;\n");

    assert_eq!(repo.rels(&Scope::default()), strings(&["f7.ts"]));
}

#[cfg(unix)]
#[test]
fn stat_leaves_links_and_intents_to_git() {
    let repo = settled_repo();

    std::os::unix::fs::symlink("a.ts", repo.path("link.ts")).unwrap();
    repo.run(&["add", "link.ts"]);
    repo.commit("link");
    assert_eq!(repo.rels(&Scope::default()), repo.git_changed());

    repo.write("intent.ts", "export {};\n");
    repo.run(&["add", "-N", "intent.ts"]);
    assert_eq!(repo.rels(&Scope::default()), repo.git_changed());
    assert_eq!(repo.rels(&Scope::default()), strings(&["intent.ts"]));
}

/// git takes a file's executable bit from its owner alone, so execute
/// permission for the group or others does not make it executable.
#[cfg(unix)]
#[test]
fn stat_reads_the_executable_bit_as_git_does() {
    use std::os::unix::fs::PermissionsExt;

    let repo = settled_repo();

    repo.write("tool.ts", "export {};\n");
    fs::set_permissions(repo.path("tool.ts"), fs::Permissions::from_mode(0o755)).unwrap();
    backdate(&repo, "tool.ts");
    repo.commit("tool");

    // Only the owner loses the bit; an index refresh that ignores modes
    // takes the new stat but keeps the executable mode. git notices the
    // change time only in a later second than the one it was added in.
    std::thread::sleep(Duration::from_millis(1100));
    fs::set_permissions(repo.path("tool.ts"), fs::Permissions::from_mode(0o655)).unwrap();
    repo.run(&["-c", "core.fileMode=false", "update-index", "-q", "--refresh"]);

    assert_eq!(repo.git_changed(), strings(&["tool.ts"]));
    assert_eq!(repo.rels(&Scope::default()), repo.git_changed());
}

#[test]
fn the_staged_set_follows_the_index_and_head() {
    let repo = repo();
    let store = repo.path(".git/fmtkit-test.dirs");

    repo.write("src/b.ts", "export const b = 2;\n").write("staged.ts", "export {};\n");
    repo.run(&["add", "src/b.ts", "staged.ts"]);
    assert_matches_git(&repo, &store, "staging");

    repo.run(&["reset", "-q", "staged.ts"]);
    assert_matches_git(&repo, &store, "unstaging");

    repo.commit("next");
    assert_matches_git(&repo, &store, "a commit");

    repo.run(&["reset", "-q", "--soft", "HEAD~1"]);
    assert_matches_git(&repo, &store, "a soft reset");
}
