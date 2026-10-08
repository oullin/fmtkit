//! End-to-end tests against a freshly built `fmtkit-go-helper`, plus fake
//! helpers written as shell scripts for the failure paths.

use std::fmt::Write;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::thread;

use fmtkit_go::{GoError, Helper, Reply, Request, Steps};

fn helper_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../go/helper")
}

/// Build the helper once per test binary, stamped with this fmtkit's version
/// so release-mode test runs pass the handshake too. `None` when `go` is
/// missing.
fn built_helper() -> Option<&'static Path> {
    static HELPER: OnceLock<Option<PathBuf>> = OnceLock::new();

    HELPER
        .get_or_init(|| {
            if Command::new("go").arg("version").output().is_err() {
                eprintln!("skipping: go is not on PATH");

                return None;
            }

            let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fmtkit-go-helper");

            let status = Command::new("go")
                .args(["build", "-ldflags", &format!("-X main.version={}", fmtkit_core::VERSION), "-o"])
                .arg(&out)
                .arg(".")
                .current_dir(helper_dir())
                .status()
                .expect("run go build");

            assert!(status.success(), "go build failed: {status}");

            Some(out)
        })
        .as_deref()
}

fn spawn() -> Option<Helper> {
    let path = built_helper()?;

    Some(Helper::spawn(Some(path)).expect("spawn helper"))
}

fn request(rel: &str, source: impl Into<Vec<u8>>, steps: Steps) -> Request {
    Request { rel: rel.into(), abs: PathBuf::from("/nonexistent").join(rel), source: source.into(), steps }
}

const SPACING: Steps = Steps { spacing: true, gofmt: false, goimports: false, resolve_imports: false, complexity: false };

const ALL: Steps = Steps { spacing: true, gofmt: true, goimports: true, resolve_imports: false, complexity: true };

#[test]
fn handshakes_and_reports_its_version() {
    let Some(helper) = spawn() else { return };

    assert_eq!(helper.version(), fmtkit_core::VERSION);
    assert!(helper.id() > 0);

    helper.shutdown().expect("clean shutdown");
}

#[test]
fn spacing_corpus_matches_the_goldens() {
    let Some(helper) = spawn() else { return };
    let corpus = helper_dir().join("spacing/testdata/corpus");
    let mut inputs: Vec<PathBuf> =
        fs::read_dir(&corpus).unwrap().map(|entry| entry.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "input")).collect();

    inputs.sort();

    assert!(!inputs.is_empty(), "no corpus fixtures in {}", corpus.display());

    let tickets: Vec<_> = inputs
        .iter()
        .map(|input| {
            let rel = input.file_name().unwrap().to_string_lossy().into_owned();

            (input.clone(), helper.submit(request(&rel, fs::read(input).unwrap(), SPACING)).unwrap())
        })
        .collect();

    for (input, ticket) in tickets {
        let reply = ticket.wait().unwrap();
        let golden = fs::read(input.with_extension("golden")).unwrap();

        assert_eq!(reply.error, None, "{}", input.display());
        assert!(reply.output == golden, "{} differs from its golden:\n{}", input.display(), String::from_utf8_lossy(&reply.output));
    }

    helper.shutdown().unwrap();
}

#[test]
fn runs_the_whole_pipeline() {
    let Some(helper) = spawn() else { return };
    let source = "package sample\nimport (\n\t\"github.com/acme/lib\"\n\t\"fmt\"\n)\nfunc run(a int) {\n\tdefer fmt.Println(lib.Name)\n\tif a > 0 {\n\t\treturn\n\t}\n}\n";
    let reply = helper.submit(request("pkg/sample.go", source, ALL)).unwrap().wait().unwrap();

    assert_eq!(reply.error, None);
    assert_eq!(reply.applied, ["spacing", "gofmt", "goimports"]);
    assert_eq!(
        String::from_utf8(reply.output).unwrap(),
        "package sample\n\nimport (\n\t\"fmt\"\n\n\t\"github.com/acme/lib\"\n)\n\nfunc run(a int) {\n\tdefer fmt.Println(lib.Name)\n\n\tif a > 0 {\n\t\treturn\n\t}\n}\n"
    );
    assert_eq!(reply.violations.len(), 1, "{:?}", reply.violations);
    assert_eq!((reply.violations[0].file.as_str(), reply.violations[0].rule.as_str(), reply.violations[0].line), ("pkg/sample.go", "spacing", 8));
    assert_eq!(reply.complexity.len(), 1);
    assert_eq!((reply.complexity[0].key.as_str(), reply.complexity[0].line), ("pkg/sample.go#run", 9));
    assert_eq!((reply.complexity[0].cyclomatic, reply.complexity[0].cognitive), (2, 1));

    helper.shutdown().unwrap();
}

#[test]
fn scores_the_shared_complexity_shapes() {
    let Some(helper) = spawn() else { return };
    let source = "package shapes\n\nfunc ifChain(a, b, c int) int {\n\tif a > 0 {\n\t\treturn 1\n\t}\n\n\tif b > 0 {\n\t\treturn 2\n\t}\n\n\tif c > 0 {\n\t\treturn 3\n\t}\n\n\treturn 0\n}\n\nfunc logicalRun(a, b, c, d bool) bool {\n\treturn a && b && c && d\n}\n\nfunc (s *shape) Read() int {\n\treturn 1\n}\n\ntype shape struct{}\n";
    let steps = Steps { complexity: true, ..Steps::default() };
    let reply = helper.submit(request("shapes.go", source, steps)).unwrap().wait().unwrap();
    let scores: Vec<_> = reply.complexity.iter().map(|s| (s.key.as_str(), s.name.as_str(), s.cyclomatic, s.cognitive)).collect();

    assert_eq!(
        scores,
        [("shapes.go#ifChain", "ifChain", 4, 3), ("shapes.go#logicalRun", "logicalRun", 4, 1), ("shapes.go#(*shape).Read", "(*shape).Read", 1, 0)]
    );

    helper.shutdown().unwrap();
}

/// The cross-language fixture, scored under its own file name as
/// `fixtures/complexity/README.md` prescribes, must match its JSON table
/// exactly, in report order.
#[test]
fn scores_the_cross_language_fixture() {
    let Some(helper) = spawn() else { return };
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/complexity");
    let source = fs::read(dir.join("shapes.go")).unwrap();
    let expected: serde_json::Value = serde_json::from_slice(&fs::read(dir.join("shapes.go.json")).unwrap()).unwrap();

    let want: Vec<(String, u64, u64, u64)> = expected
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            let number = |field: &str| entry[field].as_u64().unwrap_or_else(|| panic!("{field} in {entry}"));

            (entry["key"].as_str().unwrap().to_owned(), number("line"), number("cyclomatic"), number("cognitive"))
        })
        .collect();

    let steps = Steps { complexity: true, ..Steps::default() };
    let reply = helper.submit(request("shapes.go", source, steps)).unwrap().wait().unwrap();

    assert_eq!(reply.error, None);

    let got: Vec<(String, u64, u64, u64)> =
        reply.complexity.into_iter().map(|s| (s.key, u64::from(s.line), u64::from(s.cyclomatic), u64::from(s.cognitive))).collect();

    assert_eq!(got, want);

    helper.shutdown().unwrap();
}

#[test]
fn reports_syntax_errors_per_file() {
    let Some(helper) = spawn() else { return };
    let broken = helper.submit(request("broken.go", "package broken\nfunc (", ALL)).unwrap();
    let fine = helper.submit(request("fine.go", "package fine\n", ALL)).unwrap();
    let reply = broken.wait().unwrap();

    assert!(reply.error.as_deref().is_some_and(|e| e.starts_with("spacing: broken.go:")), "{:?}", reply.error);
    assert_eq!(reply.output, b"package broken\nfunc (");
    assert_eq!(fine.wait().unwrap().error, None);

    helper.shutdown().unwrap();
}

#[test]
fn answers_many_requests_from_many_threads() {
    let Some(helper) = spawn() else { return };

    thread::scope(|scope| {
        for t in 0..8 {
            let helper = &helper;

            scope.spawn(move || {
                let tickets: Vec<_> = (0..64)
                    .map(|i| {
                        let name = format!("f{t}x{i}");
                        let source = format!("package p\nfunc {name}(a int) int {{\n\tif a > {i} {{\n\t\treturn a\n\t}}\n\treturn {t}\n}}\n");

                        (name, helper.submit(request(&format!("t{t}/f{i}.go"), source, ALL)).unwrap())
                    })
                    .collect();

                for (i, (name, ticket)) in tickets.into_iter().enumerate() {
                    let reply = ticket.wait().unwrap();

                    assert_eq!(reply.error, None);
                    assert_eq!(reply.complexity.len(), 1);
                    assert_eq!(reply.complexity[0].key, format!("t{t}/f{i}.go#{name}"));
                    assert!(String::from_utf8_lossy(&reply.output).contains(&format!("func {name}(a int) int")));
                    assert!(reply.violations.iter().all(|v| v.file == format!("t{t}/f{i}.go")));
                }
            });
        }
    });

    helper.shutdown().unwrap();
}

#[test]
fn shutdown_answers_outstanding_requests_first() {
    let Some(helper) = spawn() else { return };
    let tickets: Vec<_> = (0..32).map(|i| helper.submit(request(&format!("f{i}.go"), "package p\nfunc f( ) {}\n", ALL)).unwrap()).collect();
    let waiter = thread::spawn(move || tickets.into_iter().map(|t| t.wait().map(|r| r.output)).collect::<Vec<_>>());

    helper.shutdown().unwrap();

    for output in waiter.join().unwrap() {
        assert_eq!(output.unwrap(), b"package p\n\nfunc f() {}\n");
    }
}

#[test]
fn killing_the_helper_fails_pending_and_later_requests() {
    let Some(helper) = spawn() else { return };
    let mut source = String::from("package big\n\n");

    for i in 0..2000 {
        writeln!(source, "func f{i}(a int) int {{\n\tif a > {i} {{\n\t\treturn a\n\t}}\n\treturn 0\n}}\n").unwrap();
    }

    let tickets: Vec<_> = (0..64).map(|i| helper.submit(request(&format!("big{i}.go"), source.clone(), ALL)).unwrap()).collect();

    let killed = Command::new("kill").args(["-KILL", &helper.id().to_string()]).status().unwrap();

    assert!(killed.success());

    let results: Vec<Result<Reply, GoError>> = tickets.into_iter().map(fmtkit_go::Ticket::wait).collect();
    let crashed = results.iter().filter(|r| matches!(r, Err(GoError::Crashed(_)))).count();

    assert!(results.iter().all(|r| r.is_ok() || matches!(r, Err(GoError::Crashed(_)))), "unexpected error");
    assert!(crashed > 0, "the kill landed after every reply");

    let message = results.iter().find_map(|r| r.as_ref().err()).unwrap().to_string();

    assert!(message.contains("go helper died: exited with signal: 9"), "{message}");

    match helper.submit(request("late.go", "package late\n", ALL)) {
        Err(GoError::Crashed(_)) => {}
        Ok(ticket) => assert!(matches!(ticket.wait(), Err(GoError::Crashed(_)))),
        Err(other) => panic!("unexpected error: {other}"),
    }

    assert!(matches!(helper.shutdown(), Err(GoError::Crashed(_))));
}

#[test]
fn dropping_the_helper_stops_it() {
    let Some(helper) = spawn() else { return };
    let pid = helper.id().to_string();

    drop(helper);

    let alive = Command::new("kill").args(["-0", &pid]).status().unwrap();

    assert!(!alive.success(), "helper {pid} survived the drop");
}

/// Write an executable shell script standing in for the helper.
///
/// The executable is a copy that `cp` writes. A file this process wrote
/// could fail to start with "Text file busy" on Linux: a child that another
/// test thread forks while the file is open for writing keeps a writable
/// handle on it until that child execs.
fn fake_helper(dir: &Path, body: &str) -> PathBuf {
    let source = dir.join("fake-helper.sh");
    let path = dir.join("fake-helper");

    fs::write(&source, format!("#!/bin/sh\n{body}\n")).unwrap();
    assert!(Command::new("cp").arg(&source).arg(&path).status().unwrap().success());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();

    path
}

#[test]
fn a_helper_that_exits_at_once_has_crashed() {
    let dir = tempfile::tempdir().unwrap();
    let path = fake_helper(dir.path(), "echo 'cannot start' >&2\nexit 3");
    let err = Helper::spawn(Some(&path)).err().expect("spawn must fail");

    assert!(matches!(&err, GoError::Crashed(m) if m.contains("exit status: 3") && m.contains("cannot start")), "{err}");
}

#[test]
fn garbage_on_stdout_is_a_protocol_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = fake_helper(dir.path(), "printf 'garbage!garbage!'\nexec sleep 30");
    let err = Helper::spawn(Some(&path)).err().expect("spawn must fail");

    assert!(matches!(&err, GoError::Protocol(m) if m.contains("exceeds")), "{err}");
}

#[test]
fn a_protocol_mismatch_is_a_version_error() {
    let dir = tempfile::tempdir().unwrap();
    // Hello{proto: 99, version: "x"}: length 14, kind 1, id 0.
    let path = fake_helper(dir.path(), "printf '\\016\\000\\000\\000\\001\\000\\000\\000\\000\\143\\000\\000\\000\\001\\000\\000\\000x'\nexec cat >/dev/null");
    let err = Helper::spawn(Some(&path)).err().expect("spawn must fail");

    assert!(matches!(&err, GoError::Version { found, .. } if found == "99 (fmtkit-go-helper x)"), "{err}");
}

#[test]
fn a_missing_helper_is_not_found() {
    let err = Helper::spawn(Some(Path::new("/nonexistent/fmtkit-go-helper"))).err().expect("spawn must fail");

    assert!(matches!(&err, GoError::NotFound(p) if p == Path::new("/nonexistent/fmtkit-go-helper")), "{err}");
}

#[test]
fn the_helper_and_tickets_cross_threads() {
    fn send_sync<T: Send + Sync>() {}
    fn send<T: Send>() {}

    send_sync::<Helper>();
    send::<fmtkit_go::Ticket>();
}
