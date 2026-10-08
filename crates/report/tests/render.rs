use fmtkit_core::{ComplexityFinding, ComplexityScore, Diagnostic, FileOutcome, Lang, Mode, Report, RunResult, Severity, VetOutcome};
use fmtkit_report::{Format, Options, Summary, render};

fn diagnostic(rule: &str, file: &str, line: u32, column: u32, message: &str, severity: Severity) -> Diagnostic {
    Diagnostic { rule: rule.into(), file: file.into(), line, column, message: message.into(), severity }
}

fn empty(mode: Mode, result: RunResult) -> Report {
    Report { schema: 2, mode, result, files: Vec::new(), complexity: Vec::new(), vet: VetOutcome::default(), missing: Vec::new() }
}

/// A changed file with a violation, a clean file, a file with lint and a
/// complexity breach, a file that failed, a stale allow entry, a vet error,
/// and a missing path.
fn sample(mode: Mode) -> Report {
    let go = FileOutcome {
        applied: vec!["spacing".into(), "gofmt".into()],
        changed: true,
        violations: vec![diagnostic("spacing", "a.go", 7, 0, "missing blank line after if statement", Severity::Error)],
        ..FileOutcome::new("a.go", Some(Lang::Go))
    };

    let ts = FileOutcome {
        lint: vec![
            diagnostic("eqeqeq", "src/b.ts", 3, 9, "Expected === and instead saw ==", Severity::Error),
            diagnostic("no-console", "src/b.ts", 10, 5, "Unexpected console statement.", Severity::Warning),
        ],
        complexity: vec![ComplexityScore { key: "src/b.ts#run".into(), name: "run".into(), line: 12, cyclomatic: 18, cognitive: 9 }],
        ..FileOutcome::new("src/b.ts", Some(Lang::Ts))
    };

    Report {
        files: vec![
            go,
            FileOutcome::new("clean.ts", Some(Lang::Ts)),
            ts,
            FileOutcome::failed("src/broken.ts", Some(Lang::Ts), "parse error: unexpected token\n  at line 2"),
        ],
        complexity: vec![
            ComplexityFinding {
                file: "fmtkit.toml".into(),
                rule: "complexity/allow".into(),
                line: 0,
                key: "old.ts#gone".into(),
                message: "allow entry old.ts#gone matches no function".into(),
            },
            ComplexityFinding {
                file: "src/b.ts".into(),
                rule: "complexity/cyclomatic".into(),
                line: 12,
                key: "src/b.ts#run".into(),
                message: "run has cyclomatic complexity 18 (limit 15)".into(),
            },
        ],
        vet: VetOutcome {
            skipped: None,
            targets: vec!["./...".into()],
            errors: vec![diagnostic("vet", "a.go", 4, 2, "fmt.Printf format %d has arg s of wrong type string", Severity::Error)],
        },
        missing: vec!["nope/".into()],
        ..empty(mode, RunResult::Fail)
    }
}

/// `a.go` was rewritten and nothing else is wrong.
fn fixed(skipped: Option<&str>) -> Report {
    let go = FileOutcome { applied: vec!["spacing".into(), "gofmt".into()], changed: true, ..FileOutcome::new("a.go", Some(Lang::Go)) };

    Report {
        files: vec![go, FileOutcome::new("b.go", Some(Lang::Go))],
        vet: VetOutcome { skipped: skipped.map(Into::into), targets: Vec::new(), errors: Vec::new() },
        ..empty(Mode::Format, RunResult::Fixed)
    }
}

fn text(report: &Report, quiet: bool) -> String {
    rendered(report, Options { format: Format::Text, color: false, quiet })
}

fn rendered(report: &Report, options: Options) -> String {
    let mut out = Vec::new();

    render(report, options, &mut out).unwrap();

    String::from_utf8(out).unwrap()
}

const SAMPLE_TEXT: &str = "  Checked 4 file(s).

  a.go
    [vet] line 4:2: fmt.Printf format %d has arg s of wrong type string
    [spacing] line 7: missing blank line after if statement
    ✓ would apply spacing, gofmt

  fmtkit.toml
    [complexity/allow] allow entry old.ts#gone matches no function

  src/b.ts
    [eqeqeq] line 3:9: Expected === and instead saw ==
    [no-console] line 10:5: warning: Unexpected console statement.
    [complexity/cyclomatic] line 12: run has cyclomatic complexity 18 (limit 15)

  src/broken.ts
    ! parse error: unexpected token
        at line 2

  ! path not found: nope/

  Result: fail. 4 file(s), 1 changed, 1 violation(s), 2 lint, 2 complexity, 1 vet, 1 error(s).
";

#[test]
fn text_groups_findings_by_file() {
    assert_eq!(text(&sample(Mode::Check), false), SAMPLE_TEXT);
}

#[test]
fn text_quiet_check_keeps_every_finding() {
    let quiet = SAMPLE_TEXT.strip_prefix("  Checked 4 file(s).\n\n").unwrap();

    assert_eq!(text(&sample(Mode::Check), true), quiet);
}

#[test]
fn text_format_mode_uses_past_tense() {
    let want = "  Formatted 2 file(s).

  a.go
    ✓ applied spacing, gofmt

  Skipped go vet: the Go toolchain is not available.

  Result: fixed. 2 file(s), 1 changed, 0 violation(s), 0 lint, 0 complexity, 0 vet, 0 error(s).
";

    assert_eq!(text(&fixed(Some("the Go toolchain is not available")), false), want);
}

#[test]
fn text_quiet_format_hides_fixed_files() {
    let want = "  Result: fixed. 2 file(s), 1 changed, 0 violation(s), 0 lint, 0 complexity, 0 vet, 0 error(s).\n";

    assert_eq!(text(&fixed(Some("no Go module")), true), want);
}

#[test]
fn text_clean_run() {
    let mut report = empty(Mode::Check, RunResult::Pass);

    report.files = vec![FileOutcome::new("a.go", Some(Lang::Go))];
    report.vet.targets = vec!["./pkg".into(), "./cmd".into()];

    let want = "  Checked 1 file(s).

  go vet passed on 2 target(s).

  Result: pass. 1 file(s), 0 changed, 0 violation(s), 0 lint, 0 complexity, 0 vet, 0 error(s).
";

    assert_eq!(text(&report, false), want);
}

#[test]
fn text_empty_scope() {
    let want = "  No files in scope.

  Result: pass. 0 file(s), 0 changed, 0 violation(s), 0 lint, 0 complexity, 0 vet, 0 error(s).
";

    assert_eq!(text(&empty(Mode::Check, RunResult::Pass), false), want);
}

#[test]
fn text_colour() {
    let mut report = empty(Mode::Check, RunResult::Fail);

    report.files = vec![FileOutcome {
        lint: vec![diagnostic("curly", "a.ts", 2, 1, "Expected { after 'if' condition.", Severity::Warning)],
        ..FileOutcome::new("a.ts", Some(Lang::Ts))
    }];

    let want = "\x1b[1;32m  Checked 1 file(s).\x1b[0m

\x1b[1;36m  a.ts\x1b[0m
    \x1b[35m[curly]\x1b[0m line 2:1: \x1b[33mwarning\x1b[0m: Expected { after 'if' condition.

\x1b[1;31m  Result: fail. 1 file(s), 0 changed, 0 violation(s), 1 lint, 0 complexity, 0 vet, 0 error(s).\x1b[0m
";

    assert_eq!(rendered(&report, Options { format: Format::Text, color: true, quiet: false }), want);
}

const SAMPLE_JSON: &str = r#"{
  "schema": 2,
  "mode": "check",
  "result": "fail",
  "summary": {
    "files": 4,
    "changed": 1,
    "violations": 1,
    "lint": 2,
    "complexity": 2,
    "vet": 1,
    "errors": 1,
    "missing": 1
  },
  "files": [
    {
      "file": "a.go",
      "lang": "go",
      "changed": true,
      "applied": [
        "spacing",
        "gofmt"
      ],
      "violations": [
        {
          "rule": "spacing",
          "line": 7,
          "message": "missing blank line after if statement",
          "severity": "error"
        }
      ]
    },
    {
      "file": "src/b.ts",
      "lang": "ts",
      "lint": [
        {
          "rule": "eqeqeq",
          "line": 3,
          "column": 9,
          "message": "Expected === and instead saw ==",
          "severity": "error"
        },
        {
          "rule": "no-console",
          "line": 10,
          "column": 5,
          "message": "Unexpected console statement.",
          "severity": "warning"
        }
      ]
    },
    {
      "file": "src/broken.ts",
      "lang": "ts",
      "error": "parse error: unexpected token\n  at line 2"
    }
  ],
  "complexity": [
    {
      "file": "fmtkit.toml",
      "rule": "complexity/allow",
      "key": "old.ts#gone",
      "message": "allow entry old.ts#gone matches no function"
    },
    {
      "file": "src/b.ts",
      "rule": "complexity/cyclomatic",
      "line": 12,
      "key": "src/b.ts#run",
      "message": "run has cyclomatic complexity 18 (limit 15)"
    }
  ],
  "vet": {
    "status": "fail",
    "targets": [
      "./..."
    ],
    "errors": [
      {
        "rule": "vet",
        "file": "a.go",
        "line": 4,
        "column": 2,
        "message": "fmt.Printf format %d has arg s of wrong type string",
        "severity": "error"
      }
    ]
  },
  "missing": [
    "nope/"
  ]
}
"#;

#[test]
fn json_projects_files_with_something_to_say() {
    assert_eq!(rendered(&sample(Mode::Check), Options { format: Format::Json, ..Options::default() }), SAMPLE_JSON);
}

#[test]
fn json_ignores_text_options() {
    let plain = rendered(&sample(Mode::Check), Options { format: Format::Json, ..Options::default() });
    let styled = rendered(&sample(Mode::Check), Options { format: Format::Json, color: true, quiet: true });

    assert_eq!(plain, styled);
}

#[test]
fn json_empty_report() {
    let want = r#"{
  "schema": 2,
  "mode": "format",
  "result": "pass",
  "summary": {
    "files": 0,
    "changed": 0,
    "violations": 0,
    "lint": 0,
    "complexity": 0,
    "vet": 0,
    "errors": 0,
    "missing": 0
  },
  "files": [],
  "complexity": [],
  "vet": {
    "status": "skipped",
    "reason": "no Go files in scope",
    "targets": [],
    "errors": []
  },
  "missing": []
}
"#;

    let mut report = empty(Mode::Format, RunResult::Pass);

    report.vet.skipped = Some("no Go files in scope".into());

    assert_eq!(rendered(&report, Options { format: Format::Json, ..Options::default() }), want);
}

#[test]
fn agent_lists_one_finding_per_line() {
    let want = "a.go:4:2 vet fmt.Printf format %d has arg s of wrong type string
a.go:7 spacing missing blank line after if statement
a.go format would apply spacing, gofmt
fmtkit.toml complexity/allow allow entry old.ts#gone matches no function
src/b.ts:3:9 eqeqeq Expected === and instead saw ==
src/b.ts:10:5 no-console warning: Unexpected console statement.
src/b.ts:12 complexity/cyclomatic run has cyclomatic complexity 18 (limit 15)
src/broken.ts error parse error: unexpected token\\n  at line 2
nope/ missing path not found
fmtkit schema=2 mode=check result=fail files=4 changed=1 violations=1 lint=2 complexity=2 vet=1 errors=1 missing=1
";

    assert_eq!(rendered(&sample(Mode::Check), Options { format: Format::Agent, ..Options::default() }), want);
}

#[test]
fn agent_format_mode_and_clean_runs() {
    let want = "a.go format applied spacing, gofmt
fmtkit schema=2 mode=format result=fixed files=2 changed=1 violations=0 lint=0 complexity=0 vet=0 errors=0 missing=0
";

    assert_eq!(rendered(&fixed(None), Options { format: Format::Agent, ..Options::default() }), want);

    let want = "fmtkit schema=2 mode=check result=pass files=0 changed=0 violations=0 lint=0 complexity=0 vet=0 errors=0 missing=0\n";

    assert_eq!(rendered(&empty(Mode::Check, RunResult::Pass), Options { format: Format::Agent, ..Options::default() }), want);
}

#[test]
fn findings_without_a_file_belong_to_the_workspace() {
    let mut report = empty(Mode::Check, RunResult::Fail);

    report.vet.targets = vec!["./...".into()];
    report.vet.errors = vec![diagnostic("vet", "", 0, 0, "go: cannot find main module", Severity::Error)];

    assert!(text(&report, true).starts_with("  workspace\n    [vet] go: cannot find main module\n\n"));
    assert!(rendered(&report, Options { format: Format::Agent, ..Options::default() }).starts_with("workspace vet go: cannot find main module\n"));
}

#[test]
fn summary_counts() {
    let summary = Summary::of(&sample(Mode::Check));

    assert_eq!(summary, Summary { files: 4, changed: 1, violations: 1, lint: 2, complexity: 2, vet: 1, errors: 1, missing: 1 });
}
