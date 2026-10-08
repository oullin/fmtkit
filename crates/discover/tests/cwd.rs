//! Runs alone in its own binary because it changes the process directory.

mod common;

use std::path::PathBuf;

use common::{Fixture, strings};
use fmtkit_discover::{Scope, find_root};

#[test]
fn a_subdirectory_cwd_still_covers_the_whole_repository() {
    let repo = Fixture::git();

    repo.write("top.ts", "1\n").write("sub/inner.ts", "1\n").write("other/x.go", "package x\n").commit("init");
    repo.write("top.ts", "2\n").write("sub/inner.ts", "2\n").write("other/new.go", "package x\n");

    std::env::set_current_dir(repo.path("sub")).unwrap();

    let root = find_root(&std::env::current_dir().unwrap());

    assert_eq!(root.canonicalize().unwrap(), repo.root().canonicalize().unwrap());
    assert_eq!(repo.rels(&Scope::default()), strings(&["other/new.go", "sub/inner.ts", "top.ts"]));
    assert_eq!(repo.rels(&Scope { all: true, ..Scope::default() }), strings(&["other/new.go", "other/x.go", "sub/inner.ts", "top.ts"]));

    let here = Scope { all: true, paths: vec![PathBuf::from(".")], ..Scope::default() };
    let parent = Scope { paths: vec![PathBuf::from("../other")], ..Scope::default() };

    assert_eq!(repo.rels(&here), strings(&["sub/inner.ts"]));
    assert_eq!(repo.rels(&parent), strings(&["other/new.go"]));
}
