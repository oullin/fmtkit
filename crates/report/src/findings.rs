use std::collections::BTreeMap;

use fmtkit_core::{Diagnostic, Report, Severity};

/// Everything a report says about one path, in display order.
pub(crate) struct Group<'a> {
    /// Repository-relative path, or `""` for findings that belong to no file.
    pub file: &'a str,
    /// Violations, lint diagnostics, complexity findings, and vet errors,
    /// ordered by line and column; file-level findings (line 0) come first.
    pub items: Vec<Item<'a>>,
    /// Why the file could not be processed.
    pub error: Option<&'a str>,
    /// The steps that changed the file, when it changed.
    pub applied: Option<&'a [String]>,
}

pub(crate) struct Item<'a> {
    pub line: u32,
    pub column: u32,
    pub rule: &'a str,
    pub message: &'a str,
    pub severity: Severity,
}

impl<'a> From<&'a Diagnostic> for Item<'a> {
    fn from(diagnostic: &'a Diagnostic) -> Self {
        Self { line: diagnostic.line, column: diagnostic.column, rule: &diagnostic.rule, message: &diagnostic.message, severity: diagnostic.severity }
    }
}

/// Every path with something to say, sorted by path. Clean files are left out.
pub(crate) fn groups(report: &Report) -> Vec<Group<'_>> {
    let mut groups: BTreeMap<&str, Group<'_>> = BTreeMap::new();

    for file in &report.files {
        if !file.changed && file.error.is_none() && file.violations.is_empty() && file.lint.is_empty() {
            continue;
        }

        let group = entry(&mut groups, &file.file);

        group.items.extend(file.violations.iter().chain(&file.lint).map(Item::from));
        group.error = file.error.as_deref();
        group.applied = file.changed.then_some(file.applied.as_slice());
    }

    for finding in &report.complexity {
        let item = Item { line: finding.line, column: 0, rule: &finding.rule, message: &finding.message, severity: Severity::Error };

        entry(&mut groups, &finding.file).items.push(item);
    }

    for error in &report.vet.errors {
        entry(&mut groups, &error.file).items.push(Item::from(error));
    }

    let mut groups: Vec<_> = groups.into_values().collect();

    for group in &mut groups {
        group.items.sort_by_key(|item| (item.line, item.column));
    }

    groups
}

fn entry<'a, 'm>(groups: &'m mut BTreeMap<&'a str, Group<'a>>, file: &'a str) -> &'m mut Group<'a> {
    groups.entry(file).or_insert_with(|| Group { file, items: Vec::new(), error: None, applied: None })
}
