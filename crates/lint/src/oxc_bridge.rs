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
    AllowWarnDeny, ConfigStore, ConfigStoreBuilder, ContextSubHost, ContextSubHostOptions, DisableDirectives, ExternalPluginStore, FixKind, LintOptions,
    LintPlugins, Message, ModuleRecord, Oxlintrc, PossibleFixes, RuleCommentType, create_unused_directives_messages, normalize_plugin_name,
    normalize_rule_name,
};
use oxc_parser::{ParseOptions, Parser};
use oxc_semantic::SemanticBuilder;
use oxc_span::{LabeledSpan, SourceType, Span};
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::{Map, Value, json};

use fmtkit_core::{Diagnostic, Edit, Lang, LineIndex, Severity};

use crate::LintError;
use crate::exempt::{self, Exempt};
use crate::rules::{self, Factory, Rule};
use crate::type_aware::{Finding, TsRule};

/// The bundled policy: v1's `.oxlintrc.json` without `jsPlugins` (those rules
/// are native here) and without `ignorePatterns`.
pub const POLICY: &str = include_str!("../policy/oxlintrc.json");

/// `[lint] type_aware = "nkzw"`: the type-aware rules `@nkzw/oxlint-config`
/// 3.0 enables, with its options, in the same shape as the policy.
pub const NKZW_TYPE_AWARE: &str = include_str!("../policy/type-aware-nkzw.json");

/// The rule program diagnostics from tsrs (a tsconfig error) are reported under.
pub const PROGRAM_RULE: &str = "typescript/program";

/// The rule name used for parse and semantic errors, which stop a file from being linted.
pub const SYNTAX_RULE: &str = "syntax";

/// The rule that reports a disable or enable directive with no effect, as
/// oxlint's `reportUnusedDisableDirectives` does. It is configured under
/// `[lint.rules]` like any other rule and takes no options.
pub const UNUSED_DIRECTIVE_RULE: &str = "unused-directive";

/// The enabled native rules, with the severity each was configured at, and
/// the files each rule, native or not, does not apply to.
#[derive(Default)]
pub struct Natives {
    pub rules: Vec<Box<dyn Rule>>,
    pub severities: FxHashMap<&'static str, Severity>,
    pub exempt: Exempt,
    /// How a directive that suppresses nothing is reported, when it is.
    pub unused_directives: Option<AllowWarnDeny>,
}

/// The configuration both linters are built from.
pub struct Built {
    pub store: ConfigStore,
    pub natives: Natives,
    pub unused_directives: Option<AllowWarnDeny>,
    /// The type-aware rules named on for some file, normalized
    /// (`typescript/no-floating-promises`), sorted.
    pub type_aware: Vec<String>,
    /// Those rules alone, overrides included, to resolve per file. The main
    /// store also enables the type-aware rules of the `correctness`
    /// category, which run only when named, as in oxlint without `typeAware`.
    pub type_store: Option<ConfigStore>,
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

    if config.type_aware == Some(fmtkit_config::TypeAwarePreset::Nkzw) {
        preset(&mut base, &mut merged, config, &registry);
    }

    for (name, setting) in &config.rules {
        if let Some(quality) = rules::quality::alias_of(name)
            && config.rules.contains_key(quality)
        {
            return Err(LintError::Rule { rule: name.clone(), message: format!("an alias of {quality}; configure one of the two names") });
        }

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

        if key == UNUSED_DIRECTIVE_RULE {
            if !options.is_empty() {
                return Err(LintError::Rule { rule: name, message: "takes no options".into() });
            }

            natives.unused_directives = severity.map(|severity| if severity == Severity::Error { AllowWarnDeny::Deny } else { AllowWarnDeny::Warn });
            continue;
        }

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

        if type_aware(&key) && options.len() > 1 {
            return Err(LintError::Rule { rule: name, message: "a type-aware rule takes at most one option".into() });
        }

        if let Owner::Plugin(plugin) = owner
            && !plugins.iter().any(|p| LintPlugins::try_from(p.as_str()).is_ok_and(|known| known == plugin))
        {
            plugins.push(<&'static str>::from(plugin).to_owned());
        }

        oxc_rules.insert(key, setting);
    }

    if config.type_aware.is_some() && !plugins.iter().any(|p| LintPlugins::try_from(p.as_str()).is_ok_and(|known| known == LintPlugins::TYPESCRIPT)) {
        plugins.push(<&'static str>::from(LintPlugins::TYPESCRIPT).to_owned());
    }

    base.insert("plugins".into(), json!(plugins));
    base.insert("rules".into(), Value::Object(oxc_rules.clone()));

    let store = build_store(&base).map_err(|message| attribute_error(&base, &oxc_rules, message))?;

    natives.exempt = exemptions(&config.exempt, &registry, &natives, &store, &base)?;

    let unused_directives = natives.unused_directives;
    let (type_aware, type_store) = type_aware_store(&base, &oxc_rules);

    Ok(Built { store, natives, unused_directives, type_aware, type_store })
}

/// The type-aware rules that are named and on, at the top level or in an
/// override, and a store of only those rules; `None` when there are none.
fn type_aware_store(base: &Map<String, Value>, oxc_rules: &Map<String, Value>) -> (Vec<String>, Option<ConfigStore>) {
    let on = |setting: &Value| {
        match setting {
            Value::Array(items) => items.first(),
            other => Some(other),
        }
        .and_then(|severity| AllowWarnDeny::try_from(severity).ok())
        .is_some_and(AllowWarnDeny::is_warn_deny)
    };
    let typed = |rules: &Map<String, Value>| -> Map<String, Value> {
        rules.iter().filter(|(name, _)| type_aware(&normalize_rule_name(name))).map(|(n, v)| (n.clone(), v.clone())).collect()
    };

    let rules = typed(oxc_rules);
    let mut named: Vec<String> = rules.iter().filter(|(_, setting)| on(setting)).map(|(name, _)| normalize_rule_name(name)).collect();
    let mut overrides = Vec::new();

    for entry in base.get("overrides").and_then(Value::as_array).into_iter().flatten() {
        let Some(own) = entry.get("rules").and_then(Value::as_object).map(typed).filter(|own| !own.is_empty()) else { continue };
        let mut entry = entry.clone();

        named.extend(own.iter().filter(|(_, setting)| on(setting)).map(|(name, _)| normalize_rule_name(name)));
        entry["rules"] = Value::Object(own);
        overrides.push(entry);
    }

    named.sort();
    named.dedup();

    if named.is_empty() {
        return (named, None);
    }

    let rc = json!({ "plugins": ["typescript"], "categories": {}, "rules": rules, "overrides": overrides });
    let store = build_store(rc.as_object().expect("an object")).expect("type-aware rules that built once build again");

    (named, Some(store))
}

/// Add the nkzw type-aware preset under the policy: its rules before
/// `[lint.rules]`, which may replace any of them, and its overrides without
/// the rules `[lint.rules]` sets, so a setting there applies to every file.
fn preset(
    base: &mut Map<String, Value>,
    merged: &mut BTreeMap<String, (String, Value)>,
    config: &fmtkit_config::Lint,
    registry: &FxHashMap<&'static str, Factory>,
) {
    let preset: Map<String, Value> = serde_json::from_str(NKZW_TYPE_AWARE).expect("the bundled preset is valid JSON");
    let configured: FxHashSet<String> = config.rules.keys().map(|name| rule_key(name, registry)).collect();

    if let Some(Value::Object(rules)) = preset.get("rules") {
        for (name, setting) in rules {
            merged.insert(rule_key(name, registry), (name.clone(), setting.clone()));
        }
    }

    for entry in preset.get("overrides").and_then(Value::as_array).into_iter().flatten() {
        let mut entry = entry.clone();

        if let Some(Value::Object(rules)) = entry.get_mut("rules") {
            rules.retain(|name, _| !configured.contains(&rule_key(name, registry)));

            if rules.is_empty() {
                continue;
            }
        }

        match base.entry("overrides").or_insert_with(|| json!([])) {
            Value::Array(overrides) => overrides.push(entry),
            other => *other = json!([entry]),
        }
    }
}

/// Check every exemption names a rule that exists and is enabled, and build
/// its matcher. Exempting a rule that is off is refused, as it is a mistake.
fn exemptions(
    entries: &[fmtkit_config::Exemption],
    registry: &FxHashMap<&'static str, Factory>,
    natives: &Natives,
    store: &ConfigStore,
    base: &Map<String, Value>,
) -> Result<Exempt, LintError> {
    let mut exempt = Exempt::default();

    if entries.is_empty() {
        return Ok(exempt);
    }

    let enabled = enabled_rules(natives, store, base);

    for entry in entries {
        let fail = |message: &str| LintError::Exempt { rule: entry.rule.clone(), message: message.to_owned() };
        let named = rule_key(&entry.rule, registry);

        if named != UNUSED_DIRECTIVE_RULE && !registry.contains_key(named.as_str()) && (matches!(owner(&named), Owner::Unknown) || !known_oxc_rule(&named)) {
            return Err(fail("unknown rule"));
        }

        // An ESLint rule may also be named `typescript/<rule>`; oxc_linter reports it as `eslint/<rule>`.
        let alias = named.strip_prefix("typescript/").filter(|rule| !enabled.contains(&named) && enabled.contains(*rule)).map(str::to_owned);
        let key = alias.unwrap_or(named);

        if !enabled.contains(&key) {
            return Err(fail("the rule is off, so there is nothing to exempt"));
        }

        let matcher = exempt::matcher(&entry.paths).map_err(|(pattern, message)| fail(&format!("{pattern}: {message}")))?;

        exempt.push(key, matcher);
    }

    Ok(exempt)
}

/// The keys of every rule that reports somewhere: the enabled native rules,
/// the oxc_linter rules the store enables (categories included), and the
/// rules an override enables for some files.
fn enabled_rules(natives: &Natives, store: &ConfigStore, base: &Map<String, Value>) -> FxHashSet<String> {
    let mut enabled: FxHashSet<String> = natives.severities.keys().map(|name| (*name).to_owned()).collect();

    if natives.unused_directives.is_some() {
        enabled.insert(UNUSED_DIRECTIVE_RULE.to_owned());
    }

    for (rule, severity) in store.rules().iter() {
        if severity.severity.is_warn_deny() {
            enabled.insert(normalize_rule_name(&format!("{}/{}", rule.plugin_name(), rule.name())));
        }
    }

    let overrides = base.get("overrides").and_then(Value::as_array).into_iter().flatten();

    for rules in overrides.filter_map(|o| o.get("rules").and_then(Value::as_object)) {
        for (name, setting) in rules {
            let on = match setting {
                Value::Array(items) => items.first(),
                other => Some(other),
            }
            .and_then(|severity| AllowWarnDeny::try_from(severity).ok())
            .is_some_and(AllowWarnDeny::is_warn_deny);

            if on {
                enabled.insert(normalize_rule_name(name));
            }
        }
    }

    enabled
}

/// The key a rule is merged under: native names as written, an `anti-slop/*`
/// alias as the `quality/*` rule it stands for, and oxc names normalized so
/// `@typescript-eslint/x`, `typescript/x` and `eslint/x` / `x` collide.
fn rule_key(name: &str, registry: &FxHashMap<&'static str, Factory>) -> String {
    if let Some(quality) = rules::quality::alias_of(name) {
        quality.to_owned()
    } else if registry.contains_key(name) || name == UNUSED_DIRECTIVE_RULE {
        name.to_owned()
    } else {
        normalize_rule_name(name)
    }
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

/// Whether oxc_linter runs this normalized rule only with type information
/// (a tsgolint rule): fmtkit's single-file lint does not run it, and tsrs does.
pub fn type_aware(key: &str) -> bool {
    let Some(rule) = key.strip_prefix("typescript/").or_else(|| key.strip_prefix("@typescript-eslint/")) else { return false };

    oxc_linter::rules::RULES.iter().any(|r| r.plugin_name() == "typescript" && r.name() == rule && r.is_tsgolint_rule())
}

/// The type-aware rule a disable directive names, by its bare name. oxc_linter
/// matches a directive name to a rule by its last `/` segment, so a bare
/// `no-floating-promises` names the type-aware rule as well as
/// `typescript/no-floating-promises` does.
fn directive_type_aware_rule(name: &str) -> Option<String> {
    let key = normalize_rule_name(name);
    let key = if type_aware(&key) || name.contains('/') { key } else { format!("typescript/{name}") };

    type_aware(&key).then(|| key.rsplit_once('/').map_or_else(|| key.clone(), |(_, bare)| bare.to_owned()))
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
/// and one that computes safe fixes. oxc_linter reports unused directives in
/// Vue files itself; [`run`] reports them for every other file.
pub fn linters(store: ConfigStore, unused_directives: Option<AllowWarnDeny>) -> (oxc_linter::Linter, oxc_linter::Linter) {
    let check = oxc_linter::Linter::new(LintOptions::default(), store.clone(), None).with_report_unused_directives(unused_directives);
    let fix = oxc_linter::Linter::new(LintOptions::default(), store, None).with_fix(FixKind::SafeFix).with_report_unused_directives(unused_directives);

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
///
/// `type_checked` says tsrs also checks this file. A directive that names no
/// rule may then suppress a tsrs finding, which this pass cannot see, so its
/// unused report carries no fix, and [`judge_type_aware`] withdraws the report
/// when the directive did suppress one.
#[allow(clippy::too_many_arguments, reason = "one pass reads every input once")]
pub fn run(oxc: &oxc_linter::Linter, natives: &Natives, allocator: &Allocator, rel: &str, lang: Lang, text: &str, fix: bool, type_checked: bool) -> Pass {
    let path = Path::new(rel);
    let index = LineIndex::new(text);
    let sections = sections(path, lang, text);
    let mut pass = Pass { diagnostics: Vec::new(), fixes: Vec::new(), syntax_error: false };
    let mut hosts = Vec::with_capacity(sections.len());
    let exempt = natives.exempt.for_file(rel);

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
            // A directive may still name a `quality/*` rule by its `anti-slop/*` alias.
            let directives = host.disable_directives();
            let suppressed = directives.contains(finding.rule, finding.span)
                || rules::quality::alias(finding.rule).is_some_and(|alias| directives.contains(&alias, finding.span));

            if exempt.native(finding.rule) || suppressed {
                continue;
            }

            let severity = natives.severities.get(finding.rule).copied().unwrap_or(Severity::Error);
            let start = finding.span.start + section.start;
            let (line, column) = index.line_col(start);

            pass.diagnostics.push(Diagnostic {
                rule: finding.name.map_or_else(|| finding.rule.to_owned(), |name| format!("{}/{name}", finding.rule)),
                file: rel.to_owned(),
                line,
                column,
                message: finding.message,
                severity,
            });

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

    // A directive counts as used when it suppressed a native finding above or
    // an oxc_linter finding here. oxc_linter returns the directives of a file
    // that is not a Vue file, whose unused ones it leaves to the caller.
    let (mut messages, directives) = oxc.run_with_disable_directives::<false>(path, hosts, allocator, None, None);

    if let (Some(severity), Some(directives)) = (natives.unused_directives, directives) {
        messages.extend(create_unused_directives_messages(&directives, severity, fix.then_some((FixKind::SafeFix, text))));
    }

    for message in messages {
        let unused = natives.unused_directives.is_some() && message.error.code.scope.is_none() && message.error.code.number.is_none();
        let mut message = message;

        // A directive for a type-aware rule is judged against what tsrs
        // reports (see `judge_type_aware`), or, when the rule is off, serves
        // a type-aware linter beside fmtkit; either way not here.
        if unused {
            match type_aware_names(text, message.span) {
                TypeAwareNames::None => {}
                TypeAwareNames::Ruleless if !type_checked => {}
                TypeAwareNames::All => continue,
                TypeAwareNames::Some | TypeAwareNames::Ruleless => message.fixes = PossibleFixes::None,
            }
        }

        let rule = if unused { UNUSED_DIRECTIVE_RULE.to_owned() } else { rule_code(&message.error) };

        if exempt.oxc(&rule) {
            continue;
        }

        if fix && let Some(unit) = oxc_fix(&message) {
            pass.fixes.push(unit);
        }

        pass.diagnostics.push(to_diagnostic(rel, &index, &message.error, message.span, rule));
    }

    pass
}

/// The type-aware rules on for one file, with the severity each runs at,
/// sorted by name. Overrides apply, as they do to every other rule.
pub fn type_aware_rules_for(store: &ConfigStore, rel: &str) -> Vec<(TsRule, Severity)> {
    let resolved = store.resolve(Path::new(rel));
    let mut rules: Vec<(TsRule, Severity)> = resolved
        .rules
        .iter()
        .filter(|(rule, config)| config.severity.is_warn_deny() && rule.is_tsgolint_rule())
        .map(|(rule, config)| {
            let options = rule.to_configuration().and_then(Result::ok);
            let severity = if config.severity == AllowWarnDeny::Deny { Severity::Error } else { Severity::Warning };

            (TsRule { name: rule.name().to_owned(), options }, severity)
        })
        .collect();

    rules.sort_by(|a, b| a.0.name.cmp(&b.0.name));
    rules
}

/// Judge what tsrs reported for one file as `run` judges in-process findings:
/// a disable directive suppresses a finding, an `[[exempt]]` entry drops it,
/// and each finding is reported as `typescript/<rule>` at its configured
/// severity.
///
/// It also judges the directives that name a type-aware rule which is on for
/// this file, which `run` leaves alone: such a directive is unused when it
/// suppressed no finding of that rule here. A directive for a type-aware rule
/// that is off may serve a type-aware linter beside fmtkit, so it is never
/// reported. The unused-directive findings carry no fix.
///
/// It also returns the line and column of each directive that names no rule
/// and suppressed a finding here, whose unused report from [`run`] stands
/// withdrawn.
pub fn judge_type_aware(natives: &Natives, rel: &str, text: &str, findings: &[Finding], rules: &[(TsRule, Severity)]) -> (Vec<Diagnostic>, Vec<(u32, u32)>) {
    let index = LineIndex::new(text);
    let exempt = natives.exempt.for_file(rel);
    let unused = natives.unused_directives.filter(|_| !exempt.oxc(UNUSED_DIRECTIVE_RULE));
    let mut out = Vec::new();
    let allocator = Allocator::default();
    let directives = text.contains("-disable").then(|| disable_directives(&allocator, rel, text)).flatten();

    if findings.is_empty() && (unused.is_none() || directives.is_none()) {
        return (out, Vec::new());
    }

    for finding in findings {
        let at = |rule: String, severity: Severity| {
            let (line, column) = index.line_col(finding.start);

            Diagnostic { rule, file: rel.to_owned(), line, column, message: finding.message.clone(), severity }
        };

        let Some(name) = &finding.rule else {
            out.push(at(PROGRAM_RULE.to_owned(), Severity::Error));
            continue;
        };

        let rule = format!("typescript/{name}");
        let suppressed = directives.as_ref().is_some_and(|host| host.disable_directives().contains(name, Span::new(finding.start, finding.end)));

        if suppressed || exempt.oxc(&rule) {
            continue;
        }

        let severity = rules.iter().find(|(r, _)| r.name == *name).map_or(Severity::Error, |(_, severity)| *severity);

        out.push(at(rule, severity));
    }

    if let (Some(severity), Some(host)) = (unused, &directives) {
        let severity = if severity == AllowWarnDeny::Deny { Severity::Error } else { Severity::Warning };

        for (span, message) in unused_type_aware_directives(host.disable_directives(), rules) {
            let (line, column) = index.line_col(span.start);

            out.push(Diagnostic { rule: UNUSED_DIRECTIVE_RULE.to_owned(), file: rel.to_owned(), line, column, message, severity });
        }
    }

    let used_ruleless = match (unused, &directives) {
        (Some(_), Some(host)) => used_ruleless_directives(text, host.disable_directives()).into_iter().map(|span| index.line_col(span.start)).collect(),
        _ => Vec::new(),
    };

    (out, used_ruleless)
}

/// The directives that name no rule and suppressed a finding, by the outer
/// span of their comment. oxc_linter keeps a ruleless comment's content span
/// (without `//` or `/*`) in `disable_rule_comments`, but reports and
/// collects unused comments by their outer span.
fn used_ruleless_directives(text: &str, directives: &DisableDirectives) -> Vec<Span> {
    let unused: FxHashSet<Span> = directives.collect_unused_disable_comments().iter().map(|comment| comment.span).collect();

    directives
        .disable_rule_comments()
        .iter()
        .filter(|comment| matches!(comment.r#type, RuleCommentType::All))
        .filter_map(|comment| {
            let start = comment.span.start.checked_sub(2)?;
            let opener = text.get(start as usize..comment.span.start as usize)?;
            let outer = match opener {
                "//" => Span::new(start, comment.span.end),
                "/*" => Span::new(start, comment.span.end + 2),
                _ => return None,
            };

            (!unused.contains(&outer)).then_some(outer)
        })
        .collect()
}

/// The names in disable directives that name a type-aware rule in `rules`
/// and suppressed nothing, with the message for each.
fn unused_type_aware_directives(directives: &DisableDirectives, rules: &[(TsRule, Severity)]) -> Vec<(Span, String)> {
    let unused = directives.collect_unused_disable_comments();
    let mut names: FxHashSet<Span> = FxHashSet::default();
    let mut comments: Vec<Span> = Vec::new();

    for comment in &unused {
        match &comment.r#type {
            RuleCommentType::All => comments.push(comment.span),
            RuleCommentType::Single(listed) => names.extend(listed.iter().map(|rule| rule.name_span)),
        }
    }

    let mut out = Vec::new();

    for comment in directives.disable_rule_comments() {
        let RuleCommentType::Single(entries) = &comment.r#type else { continue };

        for rule in entries {
            let on = directive_type_aware_rule(&rule.rule_name).is_some_and(|bare| rules.iter().any(|(r, _)| r.name == bare));
            let span = rule.name_span;

            if on && (names.contains(&span) || comments.iter().any(|c| c.start <= span.start && span.end <= c.end)) {
                out.push((span, rule.directive_prefix.unused_disable_rule_message(&rule.rule_name)));
            }
        }
    }

    out
}

/// Parse `text` far enough to read its disable directives. `None` when it
/// does not parse; its syntax errors are reported by `run`.
fn disable_directives<'a>(allocator: &'a Allocator, rel: &str, text: &'a str) -> Option<ContextSubHost<'a>> {
    let path = Path::new(rel);
    let source_type = SourceType::from_path(path).ok()?;
    let parsed = Parser::new(allocator, text, source_type)
        .with_options(ParseOptions { parse_regular_expression: true, allow_return_outside_function: true, ..ParseOptions::default() })
        .parse();

    if !parsed.diagnostics.is_empty() {
        return None;
    }

    let built = SemanticBuilder::new_linter().build(allocator.alloc(parsed.program));

    if !built.diagnostics.is_empty() {
        return None;
    }

    let record = Arc::new(ModuleRecord::new(path, &parsed.module_record, &built.semantic));

    Some(ContextSubHost::new(built.semantic, record, 0, ContextSubHostOptions::default()))
}

enum TypeAwareNames {
    /// The directive names no rule, so it covers every rule, tsrs's included.
    Ruleless,
    None,
    Some,
    All,
}

/// Which of the rules an unused-directive diagnostic covers are type-aware.
/// The diagnostic points at one rule name, or at a whole directive comment
/// when none of the rules it names suppressed anything.
fn type_aware_names(text: &str, span: Span) -> TypeAwareNames {
    let Some(covered) = text.get(span.start as usize..span.end as usize) else { return TypeAwareNames::None };
    let body = covered.trim_start_matches("//").trim_start_matches("/*").trim_end_matches("*/").trim();
    let names = match body.split_once(char::is_whitespace) {
        Some((directive, names)) if directive.starts_with("eslint-") || directive.starts_with("oxlint-") => names,
        None if body.starts_with("eslint-") || body.starts_with("oxlint-") => "",
        _ => body,
    };
    let names = directive_rule_names(names);

    if names.is_empty() {
        return TypeAwareNames::Ruleless;
    }

    let aware = names.iter().filter(|name| directive_type_aware_rule(name).is_some()).count();

    match aware {
        0 => TypeAwareNames::None,
        n if n == names.len() => TypeAwareNames::All,
        _ => TypeAwareNames::Some,
    }
}

/// The rule names in the text after a directive, as oxc_linter's
/// `collect_rule_names` reads them: separated by commas or any whitespace,
/// and ended by a description that starts with `--` or with a `-` that has
/// whitespace on both sides.
fn directive_rule_names(text: &str) -> Vec<&str> {
    let mut names = Vec::new();
    let mut start = None;
    let mut end = text.len();
    let mut previous: Option<char> = None;
    let mut chars = text.char_indices().peekable();

    while let Some((index, ch)) = chars.next() {
        let description =
            ch == '-' && chars.peek().is_some_and(|&(_, next)| next == '-' || (previous.is_some_and(char::is_whitespace) && next.is_whitespace()));

        if description {
            end = index;
            break;
        }

        if ch == ',' || ch.is_whitespace() {
            if let Some(from) = start.take() {
                names.push(&text[from..index]);
            }
        } else if start.is_none() {
            start = Some(index);
        }

        previous = Some(ch);
    }

    if let Some(from) = start {
        names.push(&text[from..end]);
    }

    names
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

/// The `plugin/rule` code an oxc_linter diagnostic is reported under.
fn rule_code(error: &OxcDiagnostic) -> String {
    match (&error.code.scope, &error.code.number) {
        (Some(scope), Some(number)) => format!("{scope}/{number}"),
        (None, Some(number)) => number.to_string(),
        (Some(scope), None) => scope.to_string(),
        (None, None) => SYNTAX_RULE.to_owned(),
    }
}

fn to_diagnostic(rel: &str, index: &LineIndex, error: &OxcDiagnostic, span: Span, rule: String) -> Diagnostic {
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
