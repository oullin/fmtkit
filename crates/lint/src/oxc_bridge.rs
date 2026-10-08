//! Everything that touches oxc_linter's API: turning the bundled policy and
//! `[lint.rules]` into an in-memory configuration, and running one lint pass
//! over a file with oxc_linter and the native rules sharing one semantic model.
//!
//! oxc_linter is a git dependency without a stable API, so nothing outside this
//! module names its types. See docs/oxc-upgrade.md.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use oxc_allocator::Allocator;
use oxc_diagnostics::{OxcDiagnostic, Severity as OxcSeverity};
use oxc_linter::loader::{JavaScriptSource, PartialLoader};
use oxc_linter::{
    ConfigStore, ConfigStoreBuilder, ContextSubHost, ContextSubHostOptions, ExternalPluginStore, FixKind, LintOptions, LintPlugins, Message, ModuleRecord,
    Oxlintrc, PossibleFixes, normalize_plugin_name, normalize_rule_name,
};
use oxc_parser::{ParseOptions, Parser};
use oxc_semantic::SemanticBuilder;
use oxc_span::{LabeledSpan, SourceType, Span};
use rustc_hash::FxHashMap;
use serde_json::{Map, Value, json};

use fmtkit_core::{Diagnostic, Edit, Lang, LineIndex, Severity};

use crate::LintError;
use crate::rules::{self, Factory, Rule};

/// The bundled policy: v1's `.oxlintrc.json` without `jsPlugins` (those rules
/// are native here) and without `ignorePatterns`.
pub const POLICY: &str = include_str!("../policy/oxlintrc.json");

/// The rule name used for parse and semantic errors, which stop a file from being linted.
pub const SYNTAX_RULE: &str = "syntax";

/// The enabled native rules, with the severity each was configured at.
#[derive(Default)]
pub struct Natives {
    pub rules: Vec<Box<dyn Rule>>,
    pub severities: FxHashMap<&'static str, Severity>,
}

/// The configuration both linters are built from.
pub struct Built {
    pub store: ConfigStore,
    pub natives: Natives,
}

/// Merge the bundled policy (when asked for) with `[lint.rules]`, route native
/// rule names to [`rules::registry`], and build oxc_linter's store from the rest.
pub fn build(config: &fmtkit_config::Lint) -> Result<Built, LintError> {
    let policy: Map<String, Value> = serde_json::from_str(POLICY).expect("the bundled policy is valid JSON");
    let registry: FxHashMap<&'static str, Factory> = rules::registry().collect();

    let mut base = if config.bundled { policy.clone() } else { Map::from_iter([("env".to_owned(), policy["env"].clone())]) };
    let mut merged: BTreeMap<String, (String, Value)> = BTreeMap::new();

    if let Some(Value::Object(policy_rules)) = base.remove("rules") {
        for (name, setting) in policy_rules {
            merged.insert(rule_key(&name, &registry), (name, setting));
        }
    }

    for (name, setting) in &config.rules {
        merged.insert(rule_key(name, &registry), (name.clone(), setting.clone()));
    }

    let mut natives = Natives::default();
    let mut oxc_rules = Map::new();
    let mut plugins: Vec<String> = match base.remove("plugins") {
        Some(Value::Array(items)) => items.into_iter().filter_map(|v| v.as_str().map(str::to_owned)).collect(),
        _ => Vec::new(),
    };

    for (key, (name, setting)) in merged {
        let (severity, options) = split_setting(&name, &setting)?;

        if let Some(factory) = registry.get(key.as_str()) {
            let Some(severity) = severity else { continue };
            let rule = factory(options).map_err(|message| LintError::Rule { rule: name.clone(), message })?;

            natives.severities.insert(rule.name(), severity);
            natives.rules.push(rule);
            continue;
        }

        let owner = owner(&key);

        if matches!(owner, Owner::Unknown) || !known_oxc_rule(&key) {
            return Err(LintError::Rule { rule: name, message: "unknown rule".into() });
        }

        if let Owner::Plugin(plugin) = owner
            && !plugins.iter().any(|p| LintPlugins::try_from(p.as_str()).is_ok_and(|known| known == plugin))
        {
            plugins.push(<&'static str>::from(plugin).to_owned());
        }

        oxc_rules.insert(key, setting);
    }

    base.insert("plugins".into(), json!(plugins));
    base.insert("rules".into(), Value::Object(oxc_rules.clone()));

    let store = build_store(&base).map_err(|message| attribute_error(&base, &oxc_rules, message))?;

    Ok(Built { store, natives })
}

/// The key a rule is merged under: native names as written, oxc names normalized
/// so `@typescript-eslint/x`, `typescript/x` and `eslint/x` / `x` collide.
fn rule_key(name: &str, registry: &FxHashMap<&'static str, Factory>) -> String {
    if registry.contains_key(name) { name.to_owned() } else { normalize_rule_name(name) }
}

/// `"off"` / `"warn"` / `"error"` / `[severity, options...]` into a severity
/// (`None` when off) and the options.
fn split_setting<'v>(name: &str, setting: &'v Value) -> Result<(Option<Severity>, &'v [Value]), LintError> {
    let invalid = || LintError::Rule { rule: name.to_owned(), message: "expected \"off\", \"warn\", \"error\", or [severity, options...]".into() };

    let (severity, options) = match setting {
        Value::Array(items) => (items.first().ok_or_else(invalid)?, &items[1..]),
        other => (other, &[][..]),
    };

    let severity = match severity {
        Value::String(s) => match s.as_str() {
            "off" | "allow" => None,
            "warn" => Some(Severity::Warning),
            "error" | "deny" => Some(Severity::Error),
            _ => return Err(invalid()),
        },
        Value::Number(n) => match n.as_u64() {
            Some(0) => None,
            Some(1) => Some(Severity::Warning),
            Some(2) => Some(Severity::Error),
            _ => return Err(invalid()),
        },
        _ => return Err(invalid()),
    };

    Ok((severity, options))
}

/// Which oxc_linter plugin a normalized rule name belongs to.
enum Owner {
    Eslint,
    Plugin(LintPlugins),
    /// A plugin oxc_linter does not ship.
    Unknown,
}

fn owner(key: &str) -> Owner {
    let Some((prefix, _)) = (if key.starts_with('@') { key.rsplit_once('/') } else { key.split_once('/') }) else {
        return Owner::Eslint;
    };

    match LintPlugins::try_from(normalize_plugin_name(prefix).as_ref()) {
        Ok(plugin) if plugin == LintPlugins::ESLINT => Owner::Eslint,
        Ok(plugin) => Owner::Plugin(plugin),
        Err(()) => Owner::Unknown,
    }
}

/// Whether oxc_linter ships a rule under this normalized name. ESLint rules that
/// oxlint adapted to TypeScript may also be named `typescript/<rule>`.
fn known_oxc_rule(key: &str) -> bool {
    let (plugin, rule) = match key.split_once('/') {
        Some((plugin, rule)) => (plugin, rule),
        None => ("eslint", key),
    };

    let internal = match plugin {
        "jsx-a11y" => "jsx_a11y",
        "react-perf" => "react_perf",
        "next" => "nextjs",
        other => other,
    };

    oxc_linter::rules::RULES.iter().any(|r| r.name() == rule && (r.plugin_name() == internal || (internal == "typescript" && r.plugin_name() == "eslint")))
}

fn build_store(rc: &Map<String, Value>) -> Result<ConfigStore, String> {
    let oxlintrc = Oxlintrc::from_json_value(&Value::Object(rc.clone())).map_err(|e| e.to_string())?;
    let mut plugins = ExternalPluginStore::new(false);
    let builder = ConfigStoreBuilder::from_oxlintrc(true, oxlintrc, None, &mut plugins, None).map_err(|e| e.to_string())?;
    let config = builder.build(&mut plugins).map_err(|e| e.to_string())?;

    Ok(ConfigStore::new(config, FxHashMap::default(), plugins))
}

/// Name the rule whose options oxc_linter rejected by building each rule alone.
fn attribute_error(base: &Map<String, Value>, oxc_rules: &Map<String, Value>, message: String) -> LintError {
    for (name, setting) in oxc_rules {
        let mut single = base.clone();

        single.insert("rules".into(), json!({ name: setting }));
        single.remove("overrides");

        if let Err(message) = build_store(&single) {
            return LintError::Rule { rule: name.clone(), message };
        }
    }

    LintError::Rule { rule: "overrides".into(), message }
}

/// Wrap a store in the two oxc linters `lint` needs: one that computes no fixes
/// and one that computes safe fixes.
pub fn linters(store: ConfigStore) -> (oxc_linter::Linter, oxc_linter::Linter) {
    let check = oxc_linter::Linter::new(LintOptions::default(), store.clone(), None);
    let fix = oxc_linter::Linter::new(LintOptions::default(), store, None).with_fix(FixKind::SafeFix);

    (check, fix)
}

/// A group of edits that has to be applied together, with the byte range it covers.
#[derive(Debug, Clone)]
pub struct FixUnit {
    pub start: u32,
    pub end: u32,
    pub edits: Vec<Edit>,
}

impl FixUnit {
    fn new(edits: Vec<Edit>) -> Option<Self> {
        let start = edits.iter().map(|e| e.start).min()?;
        let end = edits.iter().map(|e| e.end).max()?;

        Some(Self { start, end, edits })
    }
}

/// The result of one lint pass over one text.
pub struct Pass {
    pub diagnostics: Vec<Diagnostic>,
    pub fixes: Vec<FixUnit>,
    /// A script section failed to parse, so it was not linted.
    pub syntax_error: bool,
}

/// One pass over `text`: parse and build semantic once per script section, run
/// the native rules over it, then hand the same semantic to oxc_linter.
pub fn run(oxc: &oxc_linter::Linter, natives: &Natives, allocator: &Allocator, rel: &str, lang: Lang, text: &str, fix: bool) -> Pass {
    let path = Path::new(rel);
    let index = LineIndex::new(text);
    let sections = sections(path, lang, text);
    let mut pass = Pass { diagnostics: Vec::new(), fixes: Vec::new(), syntax_error: false };
    let mut hosts = Vec::with_capacity(sections.len());

    for section in sections {
        let source_text = allocator.alloc_str(section.source_text);
        let parsed = Parser::new(allocator, source_text, section.source_type)
            .with_options(ParseOptions { parse_regular_expression: true, allow_return_outside_function: true, ..ParseOptions::default() })
            .parse();

        if !parsed.diagnostics.is_empty() {
            pass.syntax_error = true;
            push_syntax_errors(&mut pass.diagnostics, rel, &index, section.start, parsed.diagnostics.into_iter());
            continue;
        }

        let built = SemanticBuilder::new_linter().build(allocator.alloc(parsed.program));

        if !built.diagnostics.is_empty() {
            pass.syntax_error = true;
            push_syntax_errors(&mut pass.diagnostics, rel, &index, section.start, built.diagnostics.into_iter());
            continue;
        }

        let mut semantic = built.semantic;

        semantic.set_irregular_whitespaces(parsed.irregular_whitespaces);

        let record = Arc::new(ModuleRecord::new(path, &parsed.module_record, &semantic));
        let mut options = ContextSubHostOptions::default();

        options.framework_options = section.framework_options;

        let host = ContextSubHost::new(semantic, record, section.start, options);

        for finding in rules::run_all(&natives.rules, host.semantic(), rel) {
            if host.disable_directives().contains(finding.rule, finding.span) {
                continue;
            }

            let severity = natives.severities.get(finding.rule).copied().unwrap_or(Severity::Error);
            let start = finding.span.start + section.start;
            let (line, column) = index.line_col(start);

            pass.diagnostics.push(Diagnostic { rule: finding.rule.to_owned(), file: rel.to_owned(), line, column, message: finding.message, severity });

            if fix {
                let edits = finding.fix.into_iter().map(|e| Edit::new(e.start + section.start, e.end + section.start, e.text)).collect();

                pass.fixes.extend(FixUnit::new(edits));
            }
        }

        hosts.push(host);
    }

    if hosts.is_empty() {
        return pass;
    }

    for message in oxc.run(path, hosts, allocator) {
        if fix && let Some(unit) = oxc_fix(&message) {
            pass.fixes.push(unit);
        }

        pass.diagnostics.push(to_diagnostic(rel, &index, &message.error, message.span));
    }

    pass
}

/// The script sections of a file: the whole file, or a Vue file's `<script>` blocks.
fn sections<'t>(path: &Path, lang: Lang, text: &'t str) -> Vec<JavaScriptSource<'t>> {
    if lang == Lang::Vue {
        return PartialLoader::parse("vue", text).unwrap_or_default();
    }

    let source_type = SourceType::from_path(path).unwrap_or_else(|_| fallback_source_type(lang));

    vec![JavaScriptSource::partial(text, source_type, 0)]
}

fn fallback_source_type(lang: Lang) -> SourceType {
    match lang {
        Lang::Tsx => SourceType::tsx(),
        Lang::Ts | Lang::Mts | Lang::Cts => SourceType::ts(),
        Lang::Jsx => SourceType::jsx(),
        Lang::Cjs => SourceType::cjs(),
        _ => SourceType::mjs(),
    }
}

/// oxlint applies the first of several possible fixes.
fn oxc_fix(message: &Message) -> Option<FixUnit> {
    let fix = match &message.fixes {
        PossibleFixes::None => return None,
        PossibleFixes::Single(fix) => fix,
        PossibleFixes::Multiple(fixes) => fixes.first()?,
    };

    if fix.span.is_unspanned() && fix.content.is_empty() {
        return None;
    }

    FixUnit::new(vec![Edit::new(fix.span.start, fix.span.end, fix.content.to_string())])
}

fn to_diagnostic(rel: &str, index: &LineIndex, error: &OxcDiagnostic, span: Span) -> Diagnostic {
    let rule = match (&error.code.scope, &error.code.number) {
        (Some(scope), Some(number)) => format!("{scope}/{number}"),
        (None, Some(number)) => number.to_string(),
        (Some(scope), None) => scope.to_string(),
        (None, None) => SYNTAX_RULE.to_owned(),
    };
    let (line, column) = index.line_col(span.start);
    let severity = if matches!(error.severity, OxcSeverity::Error) { Severity::Error } else { Severity::Warning };

    Diagnostic { rule, file: rel.to_owned(), line, column, message: error.message.to_string(), severity }
}

fn push_syntax_errors(out: &mut Vec<Diagnostic>, rel: &str, index: &LineIndex, offset: u32, errors: impl Iterator<Item = OxcDiagnostic>) {
    for error in errors {
        let start = error.labels.first().map_or(0, LabeledSpan::offset);
        let (line, column) = index.line_col(start + offset);

        out.push(Diagnostic {
            rule: SYNTAX_RULE.to_owned(),
            file: rel.to_owned(),
            line,
            column,
            message: error.message.to_string(),
            severity: Severity::Error,
        });
    }
}
