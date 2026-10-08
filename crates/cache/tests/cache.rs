use std::fs;
use std::path::Path;

use fmtkit_cache::{Cache, Key};
use fmtkit_core::{ComplexityScore, Diagnostic, FileOutcome, Lang, Mode, Severity};

const CONFIG: [u8; 32] = [7; 32];

fn outcome() -> FileOutcome {
    FileOutcome {
        lint: vec![Diagnostic {
            rule: "eqeqeq".into(),
            file: "src/a.ts".into(),
            line: 3,
            column: 9,
            message: "Expected ===".into(),
            severity: Severity::Error,
        }],
        complexity: vec![ComplexityScore { key: "src/a.ts#run".into(), name: "run".into(), line: 1, cyclomatic: 4, cognitive: 6 }],
        ..FileOutcome::new("src/a.ts", Some(Lang::Ts))
    }
}

fn open(dir: &Path) -> Cache {
    Cache::open_in(dir, Path::new("/repo"), CONFIG)
}

#[test]
fn outcomes_round_trip_through_the_store() {
    let dir = tempfile::tempdir().unwrap();
    let cache = open(dir.path());
    let key = cache.key(Mode::Check, "src/a.ts", b"let a = 1;");

    assert_eq!(cache.get(&key), None);

    cache.put(key, &outcome());

    assert_eq!(cache.get(&key), Some(outcome()));

    cache.flush().unwrap();

    let reopened = open(dir.path());

    assert_eq!(reopened.key(Mode::Check, "src/a.ts", b"let a = 1;"), key);
    assert_eq!(reopened.get(&key), Some(outcome()));
}

#[test]
fn stores_are_per_root() {
    let dir = tempfile::tempdir().unwrap();
    let one = Cache::open_in(dir.path(), Path::new("/one"), CONFIG);
    let two = Cache::open_in(dir.path(), Path::new("/two"), CONFIG);
    let key = one.key(Mode::Check, "a.ts", b"");

    assert_ne!(one.path(), two.path());
    assert!(one.path().unwrap().starts_with(dir.path()));

    one.put(key, &outcome());
    one.flush().unwrap();

    assert_eq!(two.get(&key), None);
}

#[test]
fn concurrent_gets_and_puts() {
    let dir = tempfile::tempdir().unwrap();
    let cache = open(dir.path());

    std::thread::scope(|scope| {
        for thread in 0..8u8 {
            let cache = &cache;

            scope.spawn(move || {
                for n in 0..200u8 {
                    let key = cache.key(Mode::Format, "a.ts", &[thread, n]);

                    cache.put(key, &FileOutcome::new(format!("{thread}/{n}"), None));

                    assert_eq!(cache.get(&key).unwrap().file, format!("{thread}/{n}"));
                }
            });
        }
    });

    cache.flush().unwrap();

    let reopened = open(dir.path());

    assert_eq!(reopened.get(&reopened.key(Mode::Format, "a.ts", &[7, 199])).unwrap().file, "7/199");
}

#[test]
fn corrupt_or_foreign_files_read_as_empty() {
    let dir = tempfile::tempdir().unwrap();
    let cache = open(dir.path());
    let path = cache.path().unwrap().to_path_buf();
    let key = cache.key(Mode::Check, "a.ts", b"");

    cache.put(key, &outcome());
    cache.flush().unwrap();

    let mut bytes = fs::read(&path).unwrap();

    bytes.truncate(bytes.len() / 2);
    fs::write(&path, &bytes).unwrap();

    assert_eq!(open(dir.path()).get(&key), None);

    fs::write(&path, b"not a store at all").unwrap();

    let cache = open(dir.path());

    assert_eq!(cache.get(&key), None);

    cache.put(key, &outcome());
    cache.flush().unwrap();

    assert_eq!(open(dir.path()).get(&key), Some(outcome()));
}

#[test]
fn changed_or_failed_outcomes_are_not_stored() {
    let dir = tempfile::tempdir().unwrap();
    let cache = open(dir.path());
    let changed = cache.key(Mode::Format, "a.ts", b"1");
    let failed = cache.key(Mode::Format, "b.ts", b"1");

    cache.put(changed, &FileOutcome { changed: true, ..outcome() });
    cache.put(failed, &FileOutcome::failed("b.ts", Some(Lang::Ts), "parse error"));

    assert_eq!(cache.get(&changed), None);
    assert_eq!(cache.get(&failed), None);

    let path = cache.path().unwrap().to_path_buf();

    cache.flush().unwrap();

    assert!(!path.exists());
}

#[test]
fn flush_writes_only_when_something_new_was_put() {
    let dir = tempfile::tempdir().unwrap();
    let cache = open(dir.path());
    let key = cache.key(Mode::Check, "a.ts", b"");
    let path = cache.path().unwrap().to_path_buf();

    cache.flush().unwrap();

    assert!(!path.exists());

    let cache = open(dir.path());

    cache.put(key, &outcome());
    cache.flush().unwrap();

    fs::write(&path, fs::read(&path).unwrap()).unwrap();

    let marker = fs::metadata(&path).unwrap().modified().unwrap();
    let cache = open(dir.path());

    assert!(cache.get(&key).is_some());

    cache.put(key, &outcome());
    cache.flush().unwrap();

    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), marker);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn disabled_caches_do_nothing() {
    let dir = tempfile::tempdir().unwrap();

    for cache in [Cache::disabled(), Cache::open(dir.path(), CONFIG, false)] {
        let key = cache.key(Mode::Check, "a.ts", b"content");

        assert!(!cache.is_enabled());
        assert_eq!(cache.path(), None);
        assert_eq!(key, Key([0; 32]));

        cache.put(key, &outcome());

        assert_eq!(cache.get(&key), None);

        cache.flush().unwrap();
    }

    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn keys_depend_on_every_input() {
    let dir = tempfile::tempdir().unwrap();
    let cache = open(dir.path());
    let other_config = Cache::open_in(dir.path(), Path::new("/repo"), [8; 32]);
    let base = cache.key(Mode::Check, "src/a.ts", b"let a = 1;");

    assert_eq!(base, cache.key(Mode::Check, "src/a.ts", b"let a = 1;"));
    assert_ne!(base, cache.key(Mode::Format, "src/a.ts", b"let a = 1;"));
    assert_ne!(base, cache.key(Mode::Check, "src/b.ts", b"let a = 1;"));
    assert_ne!(base, cache.key(Mode::Check, "src/a.ts", b"let a = 2;"));
    assert_ne!(base, other_config.key(Mode::Check, "src/a.ts", b"let a = 1;"));
    assert_ne!(cache.key(Mode::Check, "ab", b"c"), cache.key(Mode::Check, "a", b"bc"));
}

#[test]
fn a_config_change_misses_old_entries() {
    let dir = tempfile::tempdir().unwrap();
    let cache = open(dir.path());
    let key = cache.key(Mode::Check, "a.ts", b"");

    cache.put(key, &outcome());
    cache.flush().unwrap();

    let changed = Cache::open_in(dir.path(), Path::new("/repo"), [9; 32]);

    assert_eq!(changed.get(&changed.key(Mode::Check, "a.ts", b"")), None);
}
