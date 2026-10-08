//! Judges the scored functions against the limits and the allow list, with
//! v1's staleness rules: an allow entry is only judged by the lane that owns
//! its extension, and only when this run could have matched it, because the
//! run scored its file or the file is gone.

use std::path::Path;

use rustc_hash::FxHashSet;

use fmtkit_config::Complexity;
use fmtkit_core::{ComplexityFinding, ComplexityScore, FileOutcome, Lane, Lang, is_declaration, is_test_file};
use fmtkit_discover::SourceFile;

const RULE_CYCLOMATIC: &str = "complexity/cyclomatic";
const RULE_COGNITIVE: &str = "complexity/cognitive";
const RULE_ALLOW: &str = "complexity/allow";

pub fn evaluate(root: &Path, policy: &Complexity, lanes: &[Lane], scope: &[SourceFile], outcomes: &[FileOutcome]) -> Vec<ComplexityFinding> {
    let mut findings = Vec::new();

    for outcome in outcomes {
        for score in outcome.complexity.iter().filter(|s| !policy.allowed(&s.key)) {
            findings.extend(breach(RULE_CYCLOMATIC, &outcome.file, score, score.cyclomatic, policy.cyclomatic));
            findings.extend(breach(RULE_COGNITIVE, &outcome.file, score, score.cognitive, policy.cognitive));
        }
    }

    findings.extend(stale_entries(root, policy, lanes, scope, outcomes));
    findings.sort_by(|a, b| (&a.file, a.line, &a.rule, &a.key).cmp(&(&b.file, b.line, &b.rule, &b.key)));

    findings
}

/// The one-or-none finding a single metric produces. A limit of zero turns the metric off.
fn breach(rule: &str, file: &str, score: &ComplexityScore, value: u32, limit: u32) -> Option<ComplexityFinding> {
    (limit > 0 && value > limit).then(|| ComplexityFinding {
        file: file.to_owned(),
        rule: rule.to_owned(),
        line: score.line,
        key: score.key.clone(),
        message: format!("{} scores {value} (limit {limit})", score.key),
    })
}

fn stale_entries(root: &Path, policy: &Complexity, lanes: &[Lane], scope: &[SourceFile], outcomes: &[FileOutcome]) -> Vec<ComplexityFinding> {
    let matched: FxHashSet<&str> = outcomes.iter().flat_map(|o| o.complexity.iter().map(|s| s.key.as_str())).collect();
    let failed: FxHashSet<&str> = outcomes.iter().filter(|o| o.error.is_some()).map(|o| o.file.as_str()).collect();
    let covered: FxHashSet<&str> = scope.iter().filter(|f| is_scored(f) && !failed.contains(f.rel.as_str())).map(|f| f.rel.as_str()).collect();

    policy
        .allow
        .iter()
        .filter_map(|entry| {
            let file = entry.key.split_once('#').map_or(entry.key.as_str(), |(file, _)| file);
            let lane = Lang::from_path(Path::new(file)).filter(|l| l.is_scorable()).map(Lang::lane)?;

            if !lanes.contains(&lane) || matched.contains(entry.key.as_str()) {
                return None;
            }

            if !covered.contains(file) && root.join(file).exists() {
                return None;
            }

            Some(ComplexityFinding {
                file: file.to_owned(),
                rule: RULE_ALLOW.to_owned(),
                line: 0,
                key: entry.key.clone(),
                message: format!("allow entry {:?} matches no function", entry.key),
            })
        })
        .collect()
}

fn is_scored(file: &SourceFile) -> bool {
    file.lang.is_scorable() && !is_test_file(&file.abs) && !is_declaration(&file.abs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fmtkit_config::AllowEntry;

    fn score(key: &str, cyclomatic: u32, cognitive: u32) -> ComplexityScore {
        ComplexityScore { key: key.into(), name: key.split('#').nth(1).unwrap_or_default().into(), line: 3, cyclomatic, cognitive }
    }

    fn source(rel: &str) -> SourceFile {
        SourceFile { rel: rel.into(), abs: Path::new("/nowhere").join(rel), lang: Lang::from_path(Path::new(rel)).unwrap() }
    }

    fn policy(allow: &[&str]) -> Complexity {
        Complexity { allow: allow.iter().map(|k| AllowEntry { key: (*k).into(), reason: String::new() }).collect(), ..Complexity::default() }
    }

    #[test]
    fn reports_each_breached_metric_unless_allowed() {
        let outcome = FileOutcome { complexity: vec![score("a.ts#f", 16, 21), score("a.ts#g", 30, 30)], ..FileOutcome::new("a.ts", Some(Lang::Ts)) };
        let findings = evaluate(Path::new("/nowhere"), &policy(&["a.ts#g"]), &[Lane::Ts], &[source("a.ts")], &[outcome]);

        assert_eq!(findings.iter().map(|f| f.rule.as_str()).collect::<Vec<_>>(), [RULE_COGNITIVE, RULE_CYCLOMATIC]);
        assert_eq!(findings[0].message, "a.ts#f scores 21 (limit 20)");
    }

    #[test]
    fn judges_allow_entries_only_when_the_run_could_match_them() {
        let dir = tempfile::tempdir().unwrap();

        std::fs::write(dir.path().join("kept.go"), "package a").unwrap();

        let outcome = FileOutcome::new("a.ts", Some(Lang::Ts));
        let allow = policy(&["a.ts#gone", "kept.go#f", "deleted.go#f", "deleted.ts#f", "notes.md#x"]);
        let findings = evaluate(dir.path(), &allow, &[Lane::Ts], &[source("a.ts")], &[outcome]);
        let keys: Vec<&str> = findings.iter().map(|f| f.key.as_str()).collect();

        assert_eq!(keys, ["a.ts#gone", "deleted.ts#f"]);
    }
}
