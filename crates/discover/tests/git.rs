mod common;

use std::os::unix::fs::symlink;
use std::path::PathBuf;

use common::{Fixture, strings};
use fmtkit_config::Files;
use fmtkit_core::{Lane, Lang};
use fmtkit_discover::{DiscoverError, Scope, discover, find_root};

/// A repository with one of every kind of change on top of a first commit.
fn busy_repo() -> Fixture {
    let repo = Fixture::git();

    repo.write(".gitignore", "dist/\nignored.ts\n")
        .write("a.ts", "export const a = 1;\n")
        .write("b.go", "package b\n")
        .write("clean.ts", "export const clean = 1;\n")
        .write("src/c.tsx", "export const C = () => null;\n")
        .write("src/types.d.ts", "declare const t: number;\n")
        .write("src/gone.ts", "export {};\n")
        .write("docs/old.md", "# Old\n")
        .write("docs/kept.md", "# Kept\n")
        .write("vendor/lib/v.go", "package lib\n")
        .write("notes.txt", "not a source\n")
        .commit("init");

    repo.write("dist/forced.js", "export {};\n");
    repo.run(&["add", "-f", "dist/forced.js"]);
    repo.run(&["commit", "-q", "-m", "forced"]);

    repo.write("a.ts", "export const a = 2;\n")
        .write("b.go", "package b\n\nvar x = 1\n")
        .write("src/new.ts", "export {};\n")
        .write("newdir/deep/x.vue", "<template></template>\n")
        .write("dist/build.js", "ignored\n")
        .write("ignored.ts", "ignored\n")
        .write("x.gen.go", "package x\n")
        .write("src/more.d.ts", "declare const m: 1;\n")
        .write("node_modules/pkg/index.js", "module.exports = 1;\n")
        .write("vendor/lib/w.go", "package lib\n")
        .remove("docs/old.md");

    repo.run(&["add", "b.go"]);
    repo.run(&["mv", "src/c.tsx", "src/renamed.tsx"]);
    repo.run(&["rm", "-q", "src/gone.ts"]);

    symlink(repo.path("a.ts"), repo.path("link.ts")).unwrap();

    repo
}

#[test]
fn changed_set_matches_the_git_cli() {
    let repo = busy_repo();
    let found = repo.discover(&Scope::default());

    assert!(found.git);
    assert_eq!(found.missing, Vec::<String>::new());
    assert_eq!(repo.rels(&Scope::default()), repo.git_changed());
    assert_eq!(repo.rels(&Scope::default()), strings(&["a.ts", "b.go", "newdir/deep/x.vue", "src/new.ts", "src/renamed.tsx"]));
}

#[test]
fn all_set_matches_the_git_cli() {
    let repo = busy_repo();
    let all = Scope { all: true, ..Scope::default() };

    assert_eq!(repo.rels(&all), repo.git_all());
    assert_eq!(repo.rels(&all), strings(&["a.ts", "b.go", "clean.ts", "dist/forced.js", "docs/kept.md", "newdir/deep/x.vue", "src/new.ts", "src/renamed.tsx"]));
}

#[test]
fn files_carry_absolute_paths_and_languages() {
    let repo = busy_repo();
    let files = repo.discover(&Scope::default()).files;
    let vue = files.iter().find(|file| file.rel == "newdir/deep/x.vue").unwrap();

    assert_eq!(vue.abs, repo.path("newdir/deep/x.vue"));
    assert_eq!(vue.lang, Lang::Vue);
}

#[test]
fn staged_only_changes_are_in_scope() {
    let repo = Fixture::git();

    repo.write("a.ts", "1\n").write("b.ts", "1\n").commit("init");
    repo.write("a.ts", "2\n");
    repo.run(&["add", "a.ts"]);

    assert_eq!(repo.rels(&Scope::default()), strings(&["a.ts"]));
    assert_eq!(repo.rels(&Scope::default()), repo.git_changed());
}

#[test]
fn staged_then_deleted_files_are_left_out() {
    let repo = Fixture::git();

    repo.write("a.ts", "1\n").commit("init");
    repo.write("a.ts", "2\n");
    repo.run(&["add", "a.ts"]);
    repo.remove("a.ts");

    assert_eq!(repo.rels(&Scope::default()), Vec::<String>::new());
}

#[test]
fn clean_tree_has_an_empty_changed_set() {
    let repo = Fixture::git();

    repo.write("a.ts", "1\n").write("b.go", "package b\n").commit("init");

    assert_eq!(repo.rels(&Scope::default()), Vec::<String>::new());
    assert_eq!(repo.rels(&Scope { all: true, ..Scope::default() }), strings(&["a.ts", "b.go"]));
}

#[test]
fn unborn_head_counts_staged_and_untracked_files() {
    let repo = Fixture::git();

    repo.write("staged.ts", "1\n").write("loose.go", "package x\n");
    repo.run(&["add", "staged.ts"]);

    assert_eq!(repo.rels(&Scope::default()), strings(&["loose.go", "staged.ts"]));
    assert_eq!(repo.rels(&Scope::default()), repo.git_changed());
}

#[test]
fn lanes_filter_by_language() {
    let repo = busy_repo();
    let go = Scope { lanes: vec![Lane::Go], ..Scope::default() };
    let ts = Scope { lanes: vec![Lane::Ts], ..Scope::default() };

    assert_eq!(repo.rels(&go), strings(&["b.go"]));
    assert_eq!(repo.rels(&ts), strings(&["a.ts", "newdir/deep/x.vue", "src/new.ts", "src/renamed.tsx"]));
}

#[test]
fn paths_narrow_the_scope_and_report_missing_ones() {
    let repo = busy_repo();
    let scope = Scope { all: true, paths: vec![repo.path("src"), repo.path("docs/kept.md"), PathBuf::from("/definitely/not/here")], ..Scope::default() };
    let found = repo.discover(&scope);
    let rels: Vec<_> = found.files.iter().map(|file| file.rel.as_str()).collect();

    assert_eq!(rels, ["docs/kept.md", "src/new.ts", "src/renamed.tsx"]);
    assert_eq!(found.missing, ["/definitely/not/here"]);
}

#[test]
fn paths_cover_unchanged_files_too() {
    let repo = busy_repo();
    let scope = Scope { paths: vec![repo.path("src"), repo.path("clean.ts")], ..Scope::default() };

    assert_eq!(repo.rels(&scope), strings(&["clean.ts", "src/new.ts", "src/renamed.tsx"]));
}

#[test]
fn naming_only_tracked_files_matches_the_walk() {
    let repo = busy_repo();
    let named = ["a.ts", "clean.ts", "dist/forced.js", "vendor/lib/v.go", "notes.txt", "src/types.d.ts"];
    let tracked = Scope { paths: named.iter().map(|rel| repo.path(rel)).collect(), ..Scope::default() };
    let mut walked = tracked.clone();

    // An untracked path makes discovery walk.
    walked.paths.push(repo.path("src/new.ts"));

    assert_eq!(repo.rels(&tracked), strings(&["a.ts", "clean.ts", "dist/forced.js"]));
    assert_eq!(repo.rels(&walked), strings(&["a.ts", "clean.ts", "dist/forced.js", "src/new.ts"]));
}

#[test]
fn paths_relative_to_the_current_directory() {
    let repo = busy_repo();
    let cwd = std::env::current_dir().unwrap().canonicalize().unwrap();
    let target = repo.path("src").canonicalize().unwrap();
    let up: PathBuf = cwd.components().skip(1).map(|_| "..").collect();
    let relative = up.join(target.strip_prefix("/").unwrap());
    let scope = Scope { all: true, paths: vec![relative], ..Scope::default() };

    assert_eq!(repo.rels(&scope), strings(&["src/new.ts", "src/renamed.tsx"]));
}

#[test]
fn paths_relative_to_a_given_directory() {
    let repo = busy_repo();
    let scope = Scope { all: true, paths: vec![PathBuf::from("src"), PathBuf::from("gone")], cwd: Some(repo.root().to_path_buf()), ..Scope::default() };
    let found = repo.discover(&scope);

    assert_eq!(repo.rels(&scope), strings(&["src/new.ts", "src/renamed.tsx"]));
    // A missing path is reported as it was given.
    assert_eq!(found.missing, ["gone"]);
}

#[test]
fn the_root_as_a_path_covers_everything() {
    let repo = busy_repo();
    let scope = Scope { all: true, paths: vec![repo.root().to_path_buf()], ..Scope::default() };

    assert_eq!(repo.rels(&scope), repo.rels(&Scope { all: true, ..Scope::default() }));
}

#[test]
fn only_missing_paths_cover_nothing() {
    let repo = busy_repo();
    let found = repo.discover(&Scope { all: true, paths: vec![repo.path("nope")], ..Scope::default() });

    assert_eq!(found.files, []);
    assert_eq!(found.missing.len(), 1);
}

#[test]
fn paths_outside_the_root_are_an_error() {
    let repo = busy_repo();
    let other = tempfile::tempdir().unwrap();
    let scope = Scope { paths: vec![other.path().to_path_buf()], ..Scope::default() };

    assert!(matches!(discover(repo.root(), &scope, &Files::default()), Err(DiscoverError::Outside { .. })));
}

#[test]
fn exclude_patterns_apply_on_top_of_gitignore() {
    let repo = busy_repo();
    let files = Files { exclude: strings(&["src/", "*.vue"]) };
    let found = discover(repo.root(), &Scope { all: true, ..Scope::default() }, &files).unwrap();
    let rels: Vec<_> = found.files.iter().map(|file| file.rel.as_str()).collect();

    assert_eq!(rels, ["a.ts", "b.go", "clean.ts", "dist/forced.js", "docs/kept.md"]);
}

#[test]
fn bad_exclude_pattern_is_an_error() {
    let repo = Fixture::git();
    let files = Files { exclude: strings(&["src/[z-a]"]) };

    assert!(matches!(discover(repo.root(), &Scope::default(), &files), Err(DiscoverError::Pattern(_))));
}

#[test]
fn find_root_walks_up_to_the_work_tree() {
    let repo = busy_repo();
    let root = find_root(&repo.path("newdir/deep"));

    assert_eq!(root.canonicalize().unwrap(), repo.root().canonicalize().unwrap());
}

#[test]
fn find_root_outside_git_is_the_directory_itself() {
    let dir = Fixture::plain();

    dir.write("sub/a.ts", "1\n");

    assert_eq!(find_root(&dir.path("sub")), dir.path("sub"));
}

#[test]
fn linked_worktrees_are_their_own_root() {
    let repo = Fixture::git();
    let linked = tempfile::tempdir().unwrap();
    let path = linked.path().join("wt");

    repo.write("a.ts", "1\n").commit("init");
    repo.run(&["worktree", "add", "-q", path.to_str().unwrap()]);

    std::fs::write(path.join("b.ts"), "1\n").unwrap();

    assert_eq!(find_root(&path).canonicalize().unwrap(), path.canonicalize().unwrap());

    let found = discover(&path, &Scope::default(), &Files::default()).unwrap();
    let rels: Vec<_> = found.files.iter().map(|file| file.rel.as_str()).collect();

    assert_eq!(rels, ["b.ts"]);
}

#[test]
fn nested_repositories_are_not_entered() {
    let repo = Fixture::git();

    repo.write("a.ts", "1\n").commit("init");
    repo.write("nested/b.ts", "1\n");
    repo.run(&["-C", "nested", "init", "-q"]);

    assert!(repo.rels(&Scope { all: true, ..Scope::default() }).iter().all(|rel| !rel.starts_with("nested/")));
}

#[test]
fn unstaged_renames_report_the_new_path() {
    let repo = Fixture::git();

    repo.write("old.ts", "1\n").write("keep.ts", "1\n").commit("init");

    std::fs::rename(repo.path("old.ts"), repo.path("new.ts")).unwrap();

    assert_eq!(repo.rels(&Scope::default()), strings(&["new.ts"]));
    assert_eq!(repo.rels(&Scope::default()), repo.git_changed());
}

#[test]
fn every_ignore_source_matches_the_git_cli() {
    let repo = Fixture::git();

    repo.write(".gitignore", "*.gen.ts\n**/build/\n!keep.gen.ts\n")
        .write("src/.gitignore", "local.ts\n/anchored.ts\n")
        .write("a.ts", "1\n")
        .write("src/b.ts", "1\n")
        .commit("init");

    repo.write(".git/info/exclude", "excluded.ts\n")
        .write("x.gen.ts", "1\n")
        .write("keep.gen.ts", "1\n")
        .write("deep/build/out.ts", "1\n")
        .write("src/local.ts", "1\n")
        .write("src/deeper/local.ts", "1\n")
        .write("src/anchored.ts", "1\n")
        .write("src/deeper/anchored.ts", "1\n")
        .write("excluded.ts", "1\n")
        .write("sub/excluded.ts", "1\n")
        .write(".hidden/c.ts", "1\n");

    let all = Scope { all: true, ..Scope::default() };

    assert_eq!(repo.rels(&all), repo.git_all());
    assert_eq!(repo.rels(&Scope::default()), repo.git_changed());
    assert_eq!(repo.rels(&Scope::default()), strings(&[".hidden/c.ts", "keep.gen.ts", "src/deeper/anchored.ts"]));
}

#[test]
fn deeper_ignore_files_decide_first() {
    let repo = Fixture::git();

    repo.write(".gitignore", "*.gen.ts\n").write("src/.gitignore", "!kept.gen.ts\nlocal.ts\n").write("a.ts", "1\n").commit("init");
    repo.write("src/kept.gen.ts", "1\n").write("src/other.gen.ts", "1\n").write("local.ts", "1\n").write("src/local.ts", "1\n");

    assert_eq!(repo.rels(&Scope::default()), repo.git_changed());
    assert_eq!(repo.rels(&Scope::default()), strings(&["local.ts", "src/kept.gen.ts"]));
}

#[test]
fn linked_worktrees_follow_the_shared_exclude_file() {
    let repo = Fixture::git();

    repo.write("a.ts", "1\n").commit("init");
    repo.write(".git/info/exclude", "excluded.ts\n");

    let linked = repo.linked_worktree();
    let all = Scope { all: true, ..Scope::default() };

    linked.write("excluded.ts", "1\n").write("kept.ts", "1\n");

    assert_eq!(linked.rels(&Scope::default()), linked.git_changed());
    assert_eq!(linked.rels(&Scope::default()), strings(&["kept.ts"]));
    assert_eq!(linked.rels(&all), linked.git_all());
    assert_eq!(linked.rels(&all), strings(&["a.ts", "kept.ts"]));
}

#[test]
fn ignore_case_follows_the_repository_config() {
    let repo = Fixture::git();

    repo.write(".gitignore", "Ignored.ts\n").write("a.ts", "1\n").commit("init");
    repo.write("ignored.ts", "1\n");
    repo.run(&["config", "core.ignoreCase", "false"]);

    assert_eq!(repo.rels(&Scope::default()), repo.git_changed());
    assert_eq!(repo.rels(&Scope::default()), strings(&["ignored.ts"]));

    repo.run(&["config", "core.ignoreCase", "true"]);

    assert_eq!(repo.rels(&Scope::default()), repo.git_changed());
    assert_eq!(repo.rels(&Scope::default()), Vec::<String>::new());
}

#[test]
fn nested_repositories_are_not_untracked_changes() {
    let repo = Fixture::git();

    repo.write("a.ts", "1\n").commit("init");
    repo.write("nested/b.ts", "1\n").write("loose.ts", "1\n");
    repo.run(&["-C", "nested", "init", "-q"]);

    assert_eq!(repo.rels(&Scope::default()), strings(&["loose.ts"]));
}

#[test]
fn walking_everything_lists_go_modules() {
    let repo = Fixture::git();

    repo.write("go.mod", "module a\n")
        .write("tools/go.mod", "module t\n")
        .write("dist/go.mod", "module d\n")
        .write(".gitignore", "dist/\n")
        .write("main.go", "package main\n")
        .commit("init");

    let all = Scope { all: true, ..Scope::default() };
    let some = Scope { all: true, paths: vec![repo.path("tools")], ..Scope::default() };

    assert_eq!(repo.discover(&all).modules, strings(&["", "tools"]));
    assert_eq!(repo.discover(&some).modules, Vec::<String>::new());
    assert_eq!(repo.discover(&Scope::default()).modules, Vec::<String>::new());
}
