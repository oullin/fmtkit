use std::path::Path;

/// The languages fmtkit owns, classified by file name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Go,
    Ts,
    Tsx,
    Mts,
    Cts,
    Js,
    Jsx,
    Mjs,
    Cjs,
    Vue,
    Html,
    Markdown,
}

const DECLARATION_SUFFIXES: [&str; 3] = [".d.ts", ".d.mts", ".d.cts"];

impl Lang {
    /// Classify a path by its extension. Go files named `Dockerfile*` and the
    /// generated `*.gen.go` convention are not Go sources.
    pub fn from_path(path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_str()?;
        let ext = name.rsplit_once('.')?.1;

        let lang = match ext.to_ascii_lowercase().as_str() {
            "go" if !name.starts_with("Dockerfile") && !name.ends_with(".gen.go") => Self::Go,
            "ts" => Self::Ts,
            "tsx" => Self::Tsx,
            "mts" => Self::Mts,
            "cts" => Self::Cts,
            "js" => Self::Js,
            "jsx" => Self::Jsx,
            "mjs" => Self::Mjs,
            "cjs" => Self::Cjs,
            "vue" => Self::Vue,
            "html" | "htm" => Self::Html,
            "md" | "markdown" => Self::Markdown,
            _ => return None,
        };

        Some(lang)
    }

    /// A TypeScript or JavaScript source parsed directly by oxc.
    pub const fn is_script(self) -> bool {
        matches!(self, Self::Ts | Self::Tsx | Self::Mts | Self::Cts | Self::Js | Self::Jsx | Self::Mjs | Self::Cjs)
    }

    /// A document that embeds scripts rather than being one.
    pub const fn is_host(self) -> bool {
        matches!(self, Self::Vue | Self::Html | Self::Markdown)
    }

    /// The lane that owns this language.
    pub const fn lane(self) -> Lane {
        match self {
            Self::Go => Lane::Go,
            _ => Lane::Ts,
        }
    }

    /// Whether oxlint lints this language (scripts and Vue, not HTML or Markdown).
    pub const fn is_lintable(self) -> bool {
        self.is_script() || matches!(self, Self::Vue)
    }

    /// Whether the complexity check scores this language. `.mjs` and `.cjs`
    /// are left out, as in v1, so allow-list keys keep their meaning.
    pub const fn is_scorable(self) -> bool {
        matches!(self, Self::Go | Self::Ts | Self::Tsx | Self::Mts | Self::Cts | Self::Js | Self::Jsx)
    }
}

/// The two language lanes, selected on the CLI with `--ts` and `--go`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lane {
    Ts,
    Go,
}

/// Whether a path is a `.d.ts` family declaration file.
pub fn is_declaration(path: &Path) -> bool {
    path.file_name().and_then(|n| n.to_str()).is_some_and(|n| DECLARATION_SUFFIXES.iter().any(|s| n.ends_with(s)))
}

/// Whether a path names a test file, which the complexity check leaves out:
/// `*_test.go`, or a script whose name contains `.test.` or `.spec.`.
pub fn is_test_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };

    name.ends_with("_test.go") || name.contains(".test.") || name.contains(".spec.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_by_extension() {
        assert_eq!(Lang::from_path(Path::new("a/b.tsx")), Some(Lang::Tsx));
        assert_eq!(Lang::from_path(Path::new("main.go")), Some(Lang::Go));
        assert_eq!(Lang::from_path(Path::new("x.gen.go")), None);
        assert_eq!(Lang::from_path(Path::new("README.md")), Some(Lang::Markdown));
        assert_eq!(Lang::from_path(Path::new("Makefile")), None);
    }

    #[test]
    fn declarations_and_tests() {
        assert!(is_declaration(Path::new("x.d.ts")));
        assert!(!is_declaration(Path::new("x.ts")));
        assert!(is_test_file(Path::new("a_test.go")));
        assert!(is_test_file(Path::new("a.spec.tsx")));
    }
}
