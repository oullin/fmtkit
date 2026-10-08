//! `fmtkit serve` and the calls it answers, against the built binary.

#![cfg(unix)]

mod common;

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, SystemTime};

use common::{BIN, Fixture, assert_exit, text};

const UNFORMATTED: &str = "const  a = 1\nexport default a\n";

/// How long a server may take to start or to log a call.
const PATIENCE: Duration = Duration::from_secs(60);

/// A running `fmtkit serve` and the lines it logs.
struct Server {
    child: Child,
    lines: Receiver<String>,
}

impl Server {
    fn start(fixture: &Fixture) -> Self {
        Self::start_with(fixture.command(&["serve"]))
    }

    fn start_with(mut command: Command) -> Self {
        let mut child = command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped()).spawn().unwrap();
        let stderr = child.stderr.take().unwrap();
        let (send, lines) = mpsc::channel();

        thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                let _ = send.send(line);
            }
        });

        let server = Self { child, lines };
        let first = server.next();

        assert!(first.contains("serving"), "{first}");

        server
    }

    fn next(&self) -> String {
        self.lines.recv_timeout(PATIENCE).expect("the server logs a line")
    }

    fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.kill();
    }
}

#[test]
fn a_server_answers_as_a_local_run_would() {
    let fixture = Fixture::new(&[("app.ts", UNFORMATTED)]);
    let local = fixture.run(&["check", "--format", "agent", "--no-cache"]);
    let server = Server::start(&fixture);
    let answer = fixture.run(&["check", "--format", "agent"]);

    assert_exit(&answer, 1);
    assert!(server.next().contains("answered"));
    assert_eq!(text(&answer), text(&local));

    assert_exit(&fixture.run(&["format"]), 0);
    assert!(server.next().contains("with exit 0"));
    assert_eq!(fixture.read("app.ts"), "const a = 1;\n\nexport default a;\n");

    assert_exit(&fixture.run(&["check"]), 0);
    assert!(server.next().contains("with exit 0"));
}

#[test]
fn relative_paths_name_entries_of_the_callers_directory() {
    let fixture = Fixture::new(&[("sub/app.ts", UNFORMATTED), ("top.ts", UNFORMATTED)]);
    let server = Server::start(&fixture);
    let output = fixture.command(&["check", "--format", "agent", "app.ts"]).current_dir(fixture.path().join("sub")).output().unwrap();

    assert_exit(&output, 1);
    assert!(server.next().contains("answered"));
    assert!(text(&output).contains("sub/app.ts") && !text(&output).contains("top.ts"), "{}", text(&output));
}

#[test]
fn the_configuration_is_read_for_every_call() {
    let fixture = Fixture::new(&[("app.ts", UNFORMATTED)]);
    let server = Server::start(&fixture);

    assert_exit(&fixture.run(&["check"]), 1);
    assert!(server.next().contains("with exit 1"));

    fixture.write("fmtkit.toml", "[files]\nexclude = [\"app.ts\"]\n");

    assert_exit(&fixture.run(&["check"]), 0);
    assert!(server.next().contains("with exit 0"));
}

#[test]
fn a_call_from_another_environment_runs_locally() {
    let fixture = Fixture::new(&[("app.ts", UNFORMATTED)]);
    let server = Server::start(&fixture);
    let output = fixture.command(&["check"]).env("FMTKIT_JOBS", "1").output().unwrap();

    assert_exit(&output, 1);
    assert!(server.next().contains("refused a call: the server runs with another environment"));
}

#[test]
fn one_server_per_repository_and_a_dead_ones_socket_is_replaced() {
    let fixture = Fixture::new(&[("app.ts", UNFORMATTED)]);
    let mut first = Server::start(&fixture);
    let second = fixture.command(&["serve"]).output().unwrap();

    assert_exit(&second, 2);
    assert!(text(&second).contains("another fmtkit serve is running"), "{}", text(&second));

    // Killed, the server leaves its socket behind; nothing answers on it.
    first.kill();

    assert_exit(&fixture.run(&["check"]), 1);

    let replacement = Server::start(&fixture);

    assert_exit(&fixture.run(&["check"]), 1);
    assert!(replacement.next().contains("answered"));
}

#[test]
fn a_server_whose_executable_changed_refuses_and_stops() {
    let fixture = Fixture::new(&[("app.ts", UNFORMATTED)]);
    let bin = tempfile::tempdir().unwrap();
    let exe = bin.path().join("fmtkit");

    install(&exe, SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000));

    let server = Server::start_with(fixture.command_with(&exe, &["serve"]));

    assert_exit(&fixture.command_with(&exe, &["check"]).output().unwrap(), 1);
    assert!(server.next().contains("answered"));

    // An upgrade replaces the file; the server notices on the next call.
    install(&exe, SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000));

    assert_exit(&fixture.command_with(&exe, &["check"]).output().unwrap(), 1);
    assert!(server.next().contains("refused a call: the server's executable changed"));
    assert!(server.next().contains("stopping"));
}

/// Copy the built binary to `exe` by replacing it, dated `modified`.
fn install(exe: &Path, modified: SystemTime) {
    let staged = exe.with_extension("new");

    fs::copy(BIN, &staged).unwrap();
    fs::File::options().write(true).open(&staged).unwrap().set_modified(modified).unwrap();
    fs::rename(&staged, exe).unwrap();
}
