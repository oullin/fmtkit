//! The `fmtkit.toml` schema.
//!
//! fmtkit reads exactly one configuration file: `fmtkit.toml` at the repository
//! root, or the file `FMTKIT_CONFIG` names. Every field has a default, so an
//! absent file is a valid configuration. Unknown keys are an error.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::{env, fs, io};

use serde::{Deserialize, Serialize};

/// The file name looked up at the repository root.
pub const FILE_NAME: &str = "fmtkit.toml";

/// Names a configuration file that replaces `<root>/fmtkit.toml`.
pub const CONFIG_ENV: &str = "FMTKIT_CONFIG";

/// Caps the worker count when `--jobs` is not given.
pub const JOBS_ENV: &str = "FMTKIT_JOBS";

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("read {path}: {source}")]
    Read { path: PathBuf, source: io::Error },
    #[error("parse {path}: {source}")]
    Parse { path: PathBuf, source: Box<toml::de::Error> },
    #[error("{path}: {message}")]
    Invalid { path: PathBuf, message: String },
    #[error("{JOBS_ENV}={value} is not a whole number (0 means one worker per CPU)")]
    Jobs { value: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Worker threads; 0 means one per CPU.
    pub jobs: usize,
    pub files: Files,
    pub go: Go,
    pub ts: Ts,
    pub lint: Lint,
    pub complexity: Complexity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Files {
    /// gitignore-style patterns, matched against repository-relative paths, on
    /// top of `.gitignore`. A pattern without a slash matches at any depth.
    pub exclude: Vec<String>,
}

impl Default for Files {
    fn default() -> Self {
        Self { exclude: vec!["node_modules/".into(), "vendor/".into()] }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[allow(clippy::struct_excessive_bools)]
pub struct Go {
    /// The blank-line spacing rule, one item per line in multi-line composite
    /// literals, type ordering, and `//go:embed` repair.
    pub spacing: bool,
    pub gofmt: bool,
    pub goimports: bool,
    /// Let goimports add and remove imports, which loads packages and is slow.
    /// Off, goimports only groups and sorts the import block.
    pub resolve_imports: bool,
    pub vet: bool,
}

impl Default for Go {
    fn default() -> Self {
        Self { spacing: true, gofmt: true, goimports: true, resolve_imports: false, vet: true }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Ts {
    pub format: TsFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrailingComma {
    All,
    Es5,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArrowParens {
    Always,
    Avoid,
}

/// Printer options for scripts, also used for scripts embedded in hosts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TsFormat {
    pub use_tabs: bool,
    pub tab_width: u8,
    pub print_width: u16,
    pub single_quote: bool,
    pub semi: bool,
    pub trailing_comma: TrailingComma,
    pub arrow_parens: ArrowParens,
}

impl Default for TsFormat {
    fn default() -> Self {
        Self {
            use_tabs: true,
            tab_width: 4,
            print_width: 200,
            single_quote: true,
            semi: true,
            trailing_comma: TrailingComma::All,
            arrow_parens: ArrowParens::Always,
        }
    }
}

/// A rule's setting: a bare severity, or `[severity, options...]` as in oxlint.
pub type RuleSetting = serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Lint {
    /// Start from the bundled policy. Off, only `rules` apply.
    pub bundled: bool,
    /// Overrides keyed by rule name (`eqeqeq`, `anti-slop/no-object-parameters`).
    pub rules: BTreeMap<String, RuleSetting>,
    /// gitignore-style patterns the linter skips.
    pub ignore: Vec<String>,
    /// A TOML file of `[[exempt]]` entries, relative to the directory of the
    /// configuration file. [`Config::load_from`] appends its entries to `exempt`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exemptions: Option<String>,
    /// Rules that do not apply to some files.
    pub exempt: Vec<Exemption>,
}

impl Default for Lint {
    fn default() -> Self {
        Self { bundled: true, rules: BTreeMap::new(), ignore: Vec::new(), exemptions: None, exempt: Vec::new() }
    }
}

/// One rule that does not apply to the files `paths` matches. Every other rule
/// still applies to them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exemption {
    /// A rule name as `[lint.rules]` accepts it.
    pub rule: String,
    /// Why the rule does not apply; required, never read by the check.
    pub reason: String,
    /// gitignore-style patterns, matched against repository-relative paths as
    /// `ignore` is.
    pub paths: Vec<String>,
}

impl Exemption {
    /// The patterns that name one root-relative path: anchored by a slash
    /// before the last segment, with no wildcard, character class, brace
    /// alternative, escape or negation. A slash-free pattern matches at any
    /// depth, so it names no single path.
    pub fn exact_paths(&self) -> impl Iterator<Item = &str> {
        self.paths
            .iter()
            .map(|p| p.trim_end_matches('/'))
            .filter(|p| p.contains('/') && !p.starts_with('!') && !p.contains(['*', '?', '[', '{', '\\']))
            .map(|p| p.trim_start_matches('/'))
    }
}

/// The exemptions file named by `lint.exemptions`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ExemptionFile {
    exempt: Vec<Exemption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Complexity {
    /// Per-function cyclomatic limit; 0 turns the metric off.
    pub cyclomatic: u32,
    /// Per-function cognitive limit; 0 turns the metric off.
    pub cognitive: u32,
    /// Functions exempt from both limits until they are rewritten.
    pub allow: Vec<AllowEntry>,
}

impl Default for Complexity {
    fn default() -> Self {
        Self { cyclomatic: 15, cognitive: 20, allow: Vec::new() }
    }
}

impl Complexity {
    pub fn allowed(&self, key: &str) -> bool {
        self.allow.iter().any(|entry| entry.key == key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllowEntry {
    /// `<repo-relative path>#<function>`.
    pub key: String,
    /// Documentation only; the check never reads it.
    #[serde(default)]
    pub reason: String,
}

/// A configuration together with where it came from.
#[derive(Debug, Clone)]
pub struct Loaded {
    pub config: Config,
    /// The file it was read from, if any.
    pub path: Option<PathBuf>,
}

impl Config {
    /// Parse a configuration from TOML text. `path` is only used in errors.
    pub fn parse(text: &str, path: &Path) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(text).map_err(|source| ConfigError::Parse { path: path.to_path_buf(), source: Box::new(source) })?;

        config.validate(path)?;

        Ok(config)
    }

    /// Load `FMTKIT_CONFIG` if set, else `<root>/fmtkit.toml` if it exists, else
    /// the defaults.
    pub fn load(root: &Path) -> Result<Loaded, ConfigError> {
        let explicit = env::var_os(CONFIG_ENV).filter(|v| !v.is_empty()).map(PathBuf::from);

        Self::load_from(root, explicit.as_deref())
    }

    /// [`Config::load`] with the override passed in rather than read from the environment.
    pub fn load_from(root: &Path, explicit: Option<&Path>) -> Result<Loaded, ConfigError> {
        let path = match explicit {
            Some(path) if path.is_relative() => root.join(path),
            Some(path) => path.to_path_buf(),
            None => {
                let path = root.join(FILE_NAME);

                if !path.is_file() {
                    return Ok(Loaded { config: Self::default(), path: None });
                }

                path
            }
        };

        let text = fs::read_to_string(&path).map_err(|source| ConfigError::Read { path: path.clone(), source })?;
        let mut config = Self::parse(&text, &path)?;

        if let Some(file) = &config.lint.exemptions {
            let file = path.parent().unwrap_or(Path::new("")).join(file);
            let mut entries = read_exemptions(&file)?;

            config.lint.exempt.append(&mut entries);
        }

        Ok(Loaded { config, path: Some(path) })
    }

    fn validate(&self, path: &Path) -> Result<(), ConfigError> {
        let invalid = |message: String| Err(ConfigError::Invalid { path: path.to_path_buf(), message });

        if self.ts.format.tab_width == 0 {
            return invalid("ts.format.tab_width must be at least 1".into());
        }

        if self.ts.format.print_width == 0 {
            return invalid("ts.format.print_width must be at least 1".into());
        }

        let mut keys = std::collections::BTreeSet::new();

        for entry in &self.complexity.allow {
            if !entry.key.contains('#') {
                return invalid(format!("complexity.allow key {:?} is not <path>#<function>", entry.key));
            }

            if !keys.insert(entry.key.as_str()) {
                return invalid(format!("complexity.allow key {:?} is listed twice", entry.key));
            }
        }

        for (rule, setting) in &self.lint.rules {
            if severity_of(setting).is_none() {
                return invalid(format!("lint.rules.{rule:?}: expected \"off\", \"warn\", \"error\", or [severity, options...]"));
            }
        }

        if self.lint.exemptions.as_deref().is_some_and(|file| file.trim().is_empty()) {
            return invalid("lint.exemptions must name a file".into());
        }

        validate_exemptions(&self.lint.exempt, "lint.exempt", path)
    }

    /// The worker count from the first setting given: the CLI flag, then
    /// `FMTKIT_JOBS`, then `jobs`. 0 at any level means one worker per CPU, as
    /// it did in 0.x.
    pub fn resolve_jobs(&self, flag: Option<usize>) -> Result<usize, ConfigError> {
        let from_env = match env::var(JOBS_ENV) {
            Ok(value) if !value.is_empty() => Some(value.trim().parse::<usize>().map_err(|_| ConfigError::Jobs { value })?),
            _ => None,
        };

        match flag.or(from_env).unwrap_or(self.jobs) {
            0 => Ok(std::thread::available_parallelism().map_or(1, std::num::NonZero::get)),
            jobs => Ok(jobs),
        }
    }

    /// A stable digest of every setting that can change a file's outcome. `jobs`
    /// and `files` only change which files run and how fast, so they are left out.
    pub fn hash(&self) -> [u8; 32] {
        let relevant = (&self.go, &self.ts, &self.lint, &self.complexity.cyclomatic, &self.complexity.cognitive);
        let bytes = serde_json::to_vec(&relevant).unwrap_or_default();

        *blake3::hash(&bytes).as_bytes()
    }
}

/// Read and check the exemptions file at `path`.
fn read_exemptions(path: &Path) -> Result<Vec<Exemption>, ConfigError> {
    let text = fs::read_to_string(path).map_err(|source| ConfigError::Read { path: path.to_path_buf(), source })?;
    let file: ExemptionFile = toml::from_str(&text).map_err(|source| ConfigError::Parse { path: path.to_path_buf(), source: Box::new(source) })?;

    validate_exemptions(&file.exempt, "exempt", path)?;

    Ok(file.exempt)
}

/// Every entry names a rule, gives a reason, and lists at least one pattern.
/// Whether the rule exists and is enabled is the linter's to judge.
fn validate_exemptions(entries: &[Exemption], table: &str, path: &Path) -> Result<(), ConfigError> {
    for (at, entry) in entries.iter().enumerate() {
        let problem = if entry.rule.trim().is_empty() {
            Some("rule is empty")
        } else if entry.reason.trim().is_empty() {
            Some("reason is empty")
        } else if entry.paths.is_empty() {
            Some("paths is empty")
        } else if entry.paths.iter().any(|p| p.trim().is_empty()) {
            Some("paths has an empty pattern")
        } else {
            None
        };

        if let Some(problem) = problem {
            return Err(ConfigError::Invalid { path: path.to_path_buf(), message: format!("{table}[{at}] ({:?}): {problem}", entry.rule) });
        }
    }

    Ok(())
}

/// The severity of a rule setting, `off` / `warn` / `error`, if it is well formed.
pub fn severity_of(setting: &RuleSetting) -> Option<&str> {
    let severity = match setting {
        serde_json::Value::String(s) => s.as_str(),
        serde_json::Value::Array(items) => items.first()?.as_str()?,
        _ => return None,
    };

    matches!(severity, "off" | "allow" | "warn" | "error" | "deny").then_some(severity)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Result<Config, ConfigError> {
        Config::parse(text, Path::new("fmtkit.toml"))
    }

    #[test]
    fn empty_file_is_the_default() {
        assert_eq!(parse("").unwrap(), Config::default());
    }

    #[test]
    fn reads_every_section() {
        let config = parse(
            r#"
jobs = 4

[files]
exclude = ["dist/"]

[go]
resolve_imports = true

[ts.format]
print_width = 120
trailing_comma = "es5"

[lint.rules]
eqeqeq = ["error", "always"]
curly = "off"

[complexity]
cyclomatic = 10

[[complexity.allow]]
key = "a.go#run"
reason = "later"
"#,
        )
        .unwrap();

        assert_eq!(config.jobs, 4);
        assert_eq!(config.files.exclude, ["dist/"]);
        assert!(config.go.resolve_imports && config.go.gofmt);
        assert_eq!(config.ts.format.print_width, 120);
        assert_eq!(config.ts.format.trailing_comma, TrailingComma::Es5);
        assert_eq!(config.lint.rules["eqeqeq"], serde_json::json!(["error", "always"]));
        assert_eq!(config.complexity.cyclomatic, 10);
        assert_eq!(config.complexity.cognitive, 20);
        assert!(config.complexity.allowed("a.go#run"));
    }

    #[test]
    fn rejects_unknown_keys_and_bad_values() {
        assert!(parse("[go]\nspace = true").is_err());
        assert!(parse("[[complexity.allow]]\nkey = \"nohash\"").is_err());
        assert!(parse("[[complexity.allow]]\nkey = \"a#b\"\n[[complexity.allow]]\nkey = \"a#b\"").is_err());
        assert!(parse("[lint.rules]\ncurly = \"loud\"").is_err());
        assert!(parse("[ts.format]\ntab_width = 0").is_err());
    }

    #[test]
    fn hash_tracks_outcome_settings_only() {
        let base = Config::default();
        let jobs = Config { jobs: 3, ..Config::default() };
        let mut go = Config::default();

        go.go.gofmt = false;

        assert_eq!(base.hash(), jobs.hash());
        assert_ne!(base.hash(), go.hash());
    }

    #[test]
    fn flag_wins_over_config() {
        let config = Config { jobs: 3, ..Config::default() };

        assert_eq!(config.resolve_jobs(Some(7)).unwrap(), 7);
    }

    #[test]
    fn an_explicit_zero_means_one_worker_per_cpu() {
        let cpus = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
        let config = Config { jobs: 3, ..Config::default() };

        assert_eq!(config.resolve_jobs(Some(0)).unwrap(), cpus);
    }

    #[test]
    fn missing_file_loads_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let loaded = Config::load_from(dir.path(), None).unwrap();

        assert!(loaded.path.is_none());

        fs::write(dir.path().join("custom.toml"), "jobs = 2").unwrap();

        let loaded = Config::load_from(dir.path(), Some(Path::new("custom.toml"))).unwrap();

        assert_eq!(loaded.config.jobs, 2);
    }

    const ENTRY: &str = "rule = \"eslint/require-await\"\nreason = \"Keeps the Promise contract.\"\npaths = [\"**/*.test.ts\", \"src/a.ts\"]\n";

    #[test]
    fn reads_inline_exemptions() {
        let config = parse(&format!("[[lint.exempt]]\n{ENTRY}")).unwrap();

        assert_eq!(config.lint.exempt.len(), 1);
        assert_eq!(config.lint.exempt[0].rule, "eslint/require-await");
        assert_eq!(config.lint.exempt[0].paths, ["**/*.test.ts", "src/a.ts"]);
        assert_eq!(config.lint.exempt[0].exact_paths().collect::<Vec<_>>(), ["src/a.ts"]);

        let entry = Exemption {
            paths: ["a.ts", "/b.ts", "c/", "src/{a,b}.ts", "!src/d.ts", "e2e/", "src/f.ts"].map(String::from).into(),
            ..config.lint.exempt[0].clone()
        };

        assert_eq!(entry.exact_paths().collect::<Vec<_>>(), ["b.ts", "src/f.ts"]);
    }

    #[test]
    fn rejects_incomplete_exemptions() {
        let entry = |body: &str| parse(&format!("[[lint.exempt]]\n{body}"));

        assert!(entry("rule = \"curly\"\nreason = \"r\"\npaths = [\"a.ts\"]\nnote = \"x\"").is_err());
        assert!(entry("rule = \"curly\"\npaths = [\"a.ts\"]").is_err());
        assert!(entry("rule = \"curly\"\nreason = \" \"\npaths = [\"a.ts\"]").is_err());
        assert!(entry("rule = \"curly\"\nreason = \"r\"\npaths = []").is_err());
        assert!(entry("rule = \"\"\nreason = \"r\"\npaths = [\"a.ts\"]").is_err());
        assert!(parse("[lint]\nexemptions = \"\"").is_err());
    }

    #[test]
    fn merges_the_exemptions_file_beside_the_config() {
        let dir = tempfile::tempdir().unwrap();

        fs::create_dir(dir.path().join("config")).unwrap();
        fs::write(
            dir.path().join("config/fmtkit.toml"),
            "[lint]\nexemptions = \"exempt.toml\"\n\n[[lint.exempt]]\nrule = \"curly\"\nreason = \"r\"\npaths = [\"b.ts\"]\n",
        )
        .unwrap();
        fs::write(dir.path().join("config/exempt.toml"), format!("[[exempt]]\n{ENTRY}")).unwrap();

        let loaded = Config::load_from(dir.path(), Some(Path::new("config/fmtkit.toml"))).unwrap();
        let rules: Vec<&str> = loaded.config.lint.exempt.iter().map(|e| e.rule.as_str()).collect();

        assert_eq!(rules, ["curly", "eslint/require-await"]);
    }

    #[test]
    fn exemptions_file_errors_name_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let load = || Config::load_from(dir.path(), None).unwrap_err().to_string();

        fs::write(dir.path().join(FILE_NAME), "[lint]\nexemptions = \"exempt.toml\"\n").unwrap();

        assert!(load().contains("exempt.toml"), "{}", load());

        fs::write(dir.path().join("exempt.toml"), "[[exempt]]\nrule = \"curly\"\nreason = \"r\"\npaths = [\"a.ts\"]\nwhy = 1\n").unwrap();

        assert!(load().contains("exempt.toml") && load().contains("line 5"), "{}", load());

        fs::write(dir.path().join("exempt.toml"), "[[exempt]]\nrule = \"curly\"\nreason = \"\"\npaths = [\"a.ts\"]\n").unwrap();

        assert!(load().contains("exempt.toml") && load().contains("reason is empty"), "{}", load());
    }
}
