//! Type-aware rules end to end. The tests that need tsrs pass without running
//! when it cannot be found; set `FMTKIT_TSRS` to its native binary to run them.

mod common;

use std::path::{Path, PathBuf};

use common::{Fixture, assert_exit, text};

const API: &str = "export async function save(id: string): Promise<void> {\n\tawait Promise.resolve(id);\n}\n";
const MAIN: &str = "import { save } from './api';\n\nexport function run(): void {\n\tsave('a');\n}\n";
const SYNC_API: &str = "export function save(id: string): string {\n\treturn id;\n}\n";
const TSCONFIG: &str = "{\n\t\"compilerOptions\": { \"strict\": true, \"noEmit\": true, \"module\": \"esnext\", \"moduleResolution\": \"bundler\" },\n\t\"include\": [\"src\"]\n}\n";
const FLOATING: &str = "[lint.rules]\n\"typescript/no-floating-promises\" = \"error\"\n";

fn project(config: &str) -> Fixture {
    Fixture::new(&[("fmtkit.toml", config), ("tsconfig.json", TSCONFIG), ("src/api.ts", API), ("src/main.ts", MAIN)])
}

fn tsrs() -> Option<PathBuf> {
    let found = fmtkit_lint::type_aware::locate(Path::new(env!("CARGO_MANIFEST_DIR")), None).ok();

    if found.is_none() {
        eprintln!("skipped: no tsrs (set FMTKIT_TSRS to its native binary)");
    }

    found
}

/// Without tsrs, a run that needs it fails before it writes anything, and
/// says how to install it.
#[test]
fn a_missing_tsrs_is_a_setup_error_that_says_how_to_install_it() {
    let fixture = project(FLOATING);
    let before = fixture.snapshot();

    for args in [["check", "--all", "--ts"], ["format", "--all", "--ts"]] {
        let output = fixture.command(&args).env_remove("FMTKIT_TSRS").env("PATH", "/usr/bin:/bin").output().unwrap();

        assert_exit(&output, 2);
        assert!(text(&output).contains("npm i -D tsrs") && text(&output).contains("typescript/no-floating-promises"), "{}", text(&output));
    }

    assert_eq!(fixture.snapshot(), before);
}

#[test]
fn the_bundled_policy_needs_no_tsrs() {
    let fixture = project("");
    let output = fixture.command(&["check", "--all", "--ts", "--format", "agent"]).env_remove("FMTKIT_TSRS").env("PATH", "/usr/bin:/bin").output().unwrap();

    assert!(!text(&output).contains("tsrs"), "{}", text(&output));
    assert!(!text(&output).contains("no-floating-promises"), "{}", text(&output));
}

/// Findings come back on every run, also when the file they are in is
/// unchanged and its own outcome is cached.
#[test]
fn check_reports_type_aware_findings_that_follow_other_files() {
    let Some(tsrs) = tsrs() else { return };
    let fixture = project(FLOATING);
    let check = || fixture.command(&["check", "--all", "--ts", "--format", "agent"]).env("FMTKIT_TSRS", &tsrs).output().unwrap();

    let first = check();

    assert_exit(&first, 1);
    assert!(text(&first).contains("src/main.ts:4:2") && text(&first).contains("no-floating-promises"), "{}", text(&first));
    assert_eq!(text(&check()), text(&first), "a cached run reports the same");

    fixture.write("src/api.ts", SYNC_API);

    let after = check();

    assert!(!text(&after).contains("no-floating-promises"), "{}", text(&after));
}

#[test]
fn directives_and_exemptions_silence_type_aware_findings() {
    let Some(tsrs) = tsrs() else { return };
    let exempt = format!("{FLOATING}\n[[lint.exempt]]\nrule = \"typescript/no-floating-promises\"\nreason = \"fire and forget\"\npaths = [\"src/main.ts\"]\n");
    let check = |fixture: &Fixture| text(&fixture.command(&["check", "--all", "--ts", "--format", "agent"]).env("FMTKIT_TSRS", &tsrs).output().unwrap());

    assert!(!check(&project(&exempt)).contains("no-floating-promises"));

    let fixture = project(FLOATING);

    fixture
        .write("src/main.ts", &MAIN.replace("\tsave('a');", "\t// oxlint-disable-next-line typescript/no-floating-promises -- fire and forget\n\tsave('a');"));

    assert!(!check(&fixture).contains("no-floating-promises"), "{}", check(&fixture));
}
