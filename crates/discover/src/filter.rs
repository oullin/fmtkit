use std::path::{Path, PathBuf};

use fmtkit_core::{Lane, Lang, is_declaration};
use ignore::gitignore::{Gitignore, GitignoreBuilder};

use crate::{DiscoverError, SourceFile};

/// Directories never entered, whatever the configuration says.
const ALWAYS_EXCLUDED: [&str; 3] = [".git", "node_modules", "vendor"];

/// Everything that decides whether one repository-relative path is in scope.
pub(crate) struct Filter {
    root: PathBuf,
    exclude: Gitignore,
    lanes: Vec<Lane>,
    /// Root-relative scope paths; `None` covers the whole root.
    prefixes: Option<Vec<String>>,
}

impl Filter {
    pub(crate) fn new(root: &Path, exclude: &[String], lanes: &[Lane], prefixes: Option<Vec<String>>) -> Result<Self, DiscoverError> {
        let mut builder = GitignoreBuilder::new(root);

        for pattern in exclude {
            builder.add_line(None, pattern).map_err(|err| DiscoverError::Pattern(err.to_string()))?;
        }

        let exclude = builder.build().map_err(|err| DiscoverError::Pattern(err.to_string()))?;

        Ok(Self { root: root.to_path_buf(), exclude, lanes: lanes.to_vec(), prefixes })
    }

    /// The scope paths as git pathspecs anchored at the work tree root, or none
    /// for the whole tree.
    pub(crate) fn pathspecs(&self) -> Vec<gix::bstr::BString> {
        self.prefixes.iter().flatten().map(|prefix| format!(":(top,literal){prefix}").into()).collect()
    }

    /// The file at `rel`, if it is a source fmtkit owns and the scope covers it.
    pub(crate) fn source_file(&self, rel: &str) -> Option<SourceFile> {
        let path = Path::new(rel);
        let lang = Lang::from_path(path)?;

        if !self.lanes.is_empty() && !self.lanes.contains(&lang.lane()) {
            return None;
        }

        if is_declaration(path) || !self.covers(rel) {
            return None;
        }

        if let Some((dir, _)) = rel.rsplit_once('/')
            && dir.split('/').any(|name| ALWAYS_EXCLUDED.contains(&name))
        {
            return None;
        }

        if self.exclude.matched_path_or_any_parents(path, false).is_ignore() {
            return None;
        }

        Some(SourceFile { rel: rel.to_owned(), abs: self.root.join(path), lang })
    }

    /// Whether a walk should descend into the directory at `rel`, whose parents
    /// were all entered.
    /// The root-relative paths the scope names, or `None` for the whole root.
    pub(crate) fn prefixes(&self) -> Option<&[String]> {
        self.prefixes.as_deref()
    }

    pub(crate) fn enters(&self, rel: &str) -> bool {
        let name = rel.rsplit_once('/').map_or(rel, |(_, name)| name);

        if ALWAYS_EXCLUDED.contains(&name) {
            return false;
        }

        if let Some(prefixes) = &self.prefixes
            && !prefixes.iter().any(|prefix| is_within(rel, prefix) || is_within(prefix, rel))
        {
            return false;
        }

        !self.exclude.matched(rel, true).is_ignore()
    }

    fn covers(&self, rel: &str) -> bool {
        self.prefixes.as_ref().is_none_or(|prefixes| prefixes.iter().any(|prefix| is_within(rel, prefix)))
    }
}

/// Whether `path` is `parent` or lies below it. Everything lies below the
/// root, spelled `""`.
fn is_within(path: &str, parent: &str) -> bool {
    parent.is_empty() || path.strip_prefix(parent).is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filter(exclude: &[&str], lanes: &[Lane], prefixes: Option<&[&str]>) -> Filter {
        let exclude: Vec<String> = exclude.iter().map(|s| (*s).to_owned()).collect();
        let prefixes = prefixes.map(|p| p.iter().map(|s| (*s).to_owned()).collect());

        Filter::new(Path::new("/repo"), &exclude, lanes, prefixes).unwrap()
    }

    #[test]
    fn classifies_and_drops_declarations() {
        let f = filter(&[], &[], None);

        assert_eq!(f.source_file("a/b.ts").unwrap().lang, Lang::Ts);
        assert_eq!(f.source_file("a/b.ts").unwrap().abs, Path::new("/repo/a/b.ts"));
        assert!(f.source_file("a/b.d.ts").is_none());
        assert!(f.source_file("Makefile").is_none());
        assert!(f.source_file("x.gen.go").is_none());
    }

    #[test]
    fn always_excluded_directories() {
        let f = filter(&[], &[], None);

        assert!(f.source_file("vendor/a.go").is_none());
        assert!(f.source_file("web/node_modules/x/index.js").is_none());
        assert!(f.source_file("vendor.go").is_some());
        assert!(!f.enters("pkg/vendor"));
        assert!(f.enters("pkg/vendored"));
    }

    #[test]
    fn exclude_patterns_and_lanes() {
        let f = filter(&["dist/", "*.min.js", "/top.ts"], &[Lane::Go], None);

        assert!(f.source_file("a.ts").is_none());
        assert!(f.source_file("a.go").is_some());

        let f = filter(&["dist/", "*.min.js", "/top.ts"], &[], None);

        assert!(f.source_file("web/dist/a.ts").is_none());
        assert!(f.source_file("a.min.js").is_none());
        assert!(f.source_file("top.ts").is_none());
        assert!(f.source_file("sub/top.ts").is_some());
        assert!(!f.enters("web/dist"));
        assert!(f.enters("web"));
    }

    #[test]
    fn prefixes_limit_files_and_directories() {
        let f = filter(&[], &[], Some(&["src/app", "README.md"]));

        assert!(f.source_file("src/app/a.ts").is_some());
        assert!(f.source_file("src/application.ts").is_none());
        assert!(f.source_file("README.md").is_some());
        assert!(f.enters(""));
        assert!(f.enters("src"));
        assert!(f.enters("src/app/deep"));
        assert!(!f.enters("lib"));
        assert_eq!(f.pathspecs(), [":(top,literal)src/app", ":(top,literal)README.md"]);
    }
}
