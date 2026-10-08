mod common;

use std::os::unix::fs::symlink;

use common::{Fixture, strings};
use fmtkit_config::Files;
use fmtkit_core::Lane;
use fmtkit_discover::{Scope, discover};

fn tree() -> Fixture {
    let dir = Fixture::plain();

    dir.write(".gitignore", "build/\n*.tmp.ts\n")
        .write("a.ts", "1\n")
        .write("b.go", "package b\n")
        .write("pkg/c.tsx", "1\n")
        .write("pkg/d.d.ts", "1\n")
        .write("pkg/.gitignore", "local.ts\n")
        .write("pkg/local.ts", "1\n")
        .write("x.tmp.ts", "1\n")
        .write("build/out.js", "1\n")
        .write(".storybook/main.ts", "1\n")
        .write("node_modules/m/index.js", "1\n")
        .write("vendor/v.go", "package v\n")
        .write("README.md", "# Hi\n")
        .write("notes.txt", "1\n");

    symlink(dir.path("a.ts"), dir.path("link.ts")).unwrap();

    dir
}

#[test]
fn walks_every_file_outside_git() {
    let dir = tree();
    let found = dir.discover(&Scope::default());

    assert!(!found.git);
    assert_eq!(found.files.iter().map(|file| file.rel.as_str()).collect::<Vec<_>>(), [".storybook/main.ts", "README.md", "a.ts", "b.go", "pkg/c.tsx"]);
}

#[test]
fn all_makes_no_difference_outside_git() {
    let dir = tree();

    assert_eq!(dir.rels(&Scope { all: true, ..Scope::default() }), dir.rels(&Scope::default()));
}

#[test]
fn paths_lanes_and_excludes_apply_outside_git() {
    let dir = tree();
    let scope = Scope { paths: vec![dir.path("pkg"), dir.path("b.go"), dir.path("missing.ts")], lanes: vec![Lane::Ts], ..Scope::default() };
    let found = dir.discover(&scope);

    assert_eq!(found.files.iter().map(|file| file.rel.as_str()).collect::<Vec<_>>(), ["pkg/c.tsx"]);
    assert_eq!(found.missing, [dir.path("missing.ts").display().to_string()]);

    let files = Files { exclude: strings(&[".storybook/", "/a.ts"]) };
    let found = discover(dir.root(), &Scope::default(), &files).unwrap();

    assert_eq!(found.files.iter().map(|file| file.rel.as_str()).collect::<Vec<_>>(), ["README.md", "b.go", "pkg/c.tsx"]);
}
