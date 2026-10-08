//! Goldens through the real pipeline: every `tests/fixtures/<name>.<ext>` must
//! format to `<name>.formatted.<ext>`, which must itself be stable.
//!
//! Run with `FMTKIT_BLESS=1` to rewrite the expected files after an intended
//! change (for instance when the TS pipeline behind embedded scripts changes).

use std::fs;
use std::path::{Path, PathBuf};

use fmtkit_config::TsFormat;
use fmtkit_core::Lang;
use fmtkit_hosts::{Formatted, format_host};
use proptest::prelude::*;

fn fixtures() -> Vec<(PathBuf, Lang)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");

    let mut inputs: Vec<(PathBuf, Lang)> = fs::read_dir(&dir)
        .expect("fixtures directory")
        .map(|entry| entry.expect("fixture entry").path())
        .filter(|path| !path.to_string_lossy().contains(".formatted."))
        .filter_map(|path| Lang::from_path(&path).filter(|lang| lang.is_host()).map(|lang| (path, lang)))
        .collect();

    inputs.sort();
    inputs
}

fn expected_path(input: &Path) -> PathBuf {
    let stem = input.file_stem().and_then(|s| s.to_str()).expect("fixture stem");
    let ext = input.extension().and_then(|s| s.to_str()).expect("fixture extension");

    input.with_file_name(format!("{stem}.formatted.{ext}"))
}

fn format(lang: Lang, source: &str) -> Formatted {
    format_host("fixture", lang, source, &TsFormat::default()).unwrap_or_else(|error| panic!("{error}"))
}

#[test]
fn fixtures_match_their_goldens() {
    let bless = std::env::var_os("FMTKIT_BLESS").is_some();
    let fixtures = fixtures();

    assert!(fixtures.len() >= 3, "expected Vue, HTML and Markdown fixtures");

    for (input, lang) in fixtures {
        let source = fs::read_to_string(&input).expect("fixture");
        let formatted = format(lang, &source);
        let expected_path = expected_path(&input);

        if bless {
            fs::write(&expected_path, &formatted.output).expect("write golden");

            continue;
        }

        let expected = fs::read_to_string(&expected_path).unwrap_or_else(|_| panic!("missing {}; run with FMTKIT_BLESS=1", expected_path.display()));

        assert_eq!(formatted.output, expected, "{} differs from its golden", input.display());
        assert!(!formatted.applied.is_empty(), "{}", input.display());
    }
}

#[test]
fn goldens_are_stable() {
    for (input, lang) in fixtures() {
        let Ok(expected) = fs::read_to_string(expected_path(&input)) else {
            continue;
        };

        let again = format(lang, &expected);

        assert_eq!(again.output, expected, "{} is not idempotent", input.display());
        assert!(again.applied.is_empty(), "{}: {:?}", input.display(), again.applied);
    }
}

#[test]
fn crlf_fixtures_format_like_lf_fixtures() {
    for (input, lang) in fixtures() {
        let source = fs::read_to_string(&input).expect("fixture");

        assert_eq!(format(lang, &source.replace('\n', "\r\n")).output, format(lang, &source).output, "{}", input.display());
    }
}

/// A fixture with some of its lines re-indented, joined with CRLF, or cut
/// short of its trailing newline.
fn mutated_fixture() -> impl Strategy<Value = (Lang, String)> {
    let fixtures = fixtures();
    let count = fixtures.len();

    (0..count, prop::collection::vec((any::<bool>(), 0usize..3), 0..64), any::<bool>(), any::<bool>()).prop_map(move |(index, edits, crlf, trim)| {
        let (path, lang) = &fixtures[index];
        let source = fs::read_to_string(path).expect("fixture");
        let mut lines: Vec<String> = source.lines().map(str::to_owned).collect();

        for (line, (indent, spaces)) in lines.iter_mut().zip(edits) {
            if indent && *lang != Lang::Markdown {
                line.insert_str(0, &" ".repeat(spaces));
            }
        }

        let mut text = lines.join(if crlf { "\r\n" } else { "\n" });

        if !trim {
            text.push_str(if crlf { "\r\n" } else { "\n" });
        }

        (*lang, text)
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn mutated_fixtures_format_idempotently((lang, source) in mutated_fixture()) {
        let once = format(lang, &source);
        let twice = format(lang, &once.output);

        prop_assert_eq!(&twice.output, &once.output);
        prop_assert!(twice.applied.is_empty(), "{:?}", twice.applied);
    }
}
