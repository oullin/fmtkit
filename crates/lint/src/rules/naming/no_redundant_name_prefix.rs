//! `naming/no-redundant-name-prefix`: a folder, file or module-level
//! declaration whose name starts with what its path already says.
//!
//! The path says the workspace's folder name and the name of every folder
//! between the workspace and the file, each also in the singular. Inside
//! `packages/twilio/src/verify/`, `twilio-verify-driver.ts` repeats `twilio`
//! and `verify`, and becomes `driver.ts`. A name that is only the prefix, such
//! as `provider.ts` in `providers`, says something on its own and is kept.
//!
//! An exported declaration is read where it is imported, beside its
//! workspace's import specifier but not its folders, so only the workspace
//! name counts, and the prefix is kept when trimming would leave one word:
//! `TwilioLookupDriver` becomes `LookupDriver`, while `AccessProvider` stays,
//! because `Provider` alone would clash where the workspaces meet.
//!
//! The rule reads one file at a time and has no default exemptions: product
//! and vendor terms (`terms`), folder names a tool requires (`folders`) and
//! skipped paths (`exempt`) all come from its options.

use oxc_ast::ast::{BindingIdentifier, Declaration, ExportDefaultDeclarationKind, ImportDeclarationSpecifier, Statement};
use oxc_span::Span;
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::Value;

use super::super::boundaries::{Paths, authored};
use super::super::{Context, Rule};
use super::words::{self, Word};

pub const NAME: &str = "naming/no-redundant-name-prefix";

/// `[{ workspaces: [glob], terms?: [name], folders?: [name], exempt?: [glob] }]`.
pub fn build(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    let [Value::Object(object)] = options else {
        return Err("expected one options object with \"workspaces\"".into());
    };

    let mut rule = NoRedundantNamePrefix { workspaces: Paths::default(), terms: Vec::new(), folders: FxHashSet::default(), exempt: Paths::default() };
    let mut workspaces = false;

    for (key, value) in object {
        match key.as_str() {
            "workspaces" => {
                let patterns = names(key, value)?;

                workspaces = !patterns.is_empty();
                rule.workspaces = Paths::parse(key, &patterns)?;
            }
            "terms" => rule.terms = names(key, value)?.iter().map(|term| words::phrase(term)).collect(),
            "folders" => rule.folders = names(key, value)?.into_iter().collect(),
            "exempt" => rule.exempt = Paths::parse(key, &names(key, value)?)?,
            other => return Err(format!("unknown option {other:?}")),
        }
    }

    if !workspaces {
        return Err("workspaces is required and must not be empty".into());
    }
    if rule.terms.iter().any(Vec::is_empty) {
        return Err("terms: an entry has no letters or digits".into());
    }

    Ok(Box::new(rule))
}

fn names(key: &str, value: &Value) -> Result<Vec<String>, String> {
    value
        .as_array()
        .ok_or_else(|| format!("{key} must be an array of strings"))?
        .iter()
        .map(|item| {
            item.as_str().filter(|item| !item.trim().is_empty()).map(str::to_owned).ok_or_else(|| format!("{key} must be an array of non-blank strings"))
        })
        .collect()
}

struct NoRedundantNamePrefix {
    workspaces: Paths,
    /// Product and vendor terms, as phrases, that a trim must not cut apart.
    terms: Vec<Vec<String>>,
    /// Folder names a tool requires, never reported.
    folders: FxHashSet<String>,
    exempt: Paths,
}

/// A name with its redundant prefix removed.
struct Trim {
    prefix: String,
    suggestion: String,
}

impl Rule for NoRedundantNamePrefix {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run_once(&self, ctx: &mut Context<'_, '_>) {
        let rel = ctx.rel;

        if self.exempt.matches(rel) || !authored(rel) {
            return;
        }

        let segments: Vec<_> = rel.split('/').collect();
        let Some(depth) = (1..segments.len()).rev().find(|&depth| self.workspaces.matches(&segments[..depth].join("/"))) else {
            return;
        };
        let workspace = words::forms(segments[depth - 1]);
        let mut path = workspace.clone();

        for (index, folder) in segments.iter().enumerate().take(segments.len() - 1).skip(depth) {
            if !self.folders.contains(*folder)
                && let Some(trim) = self.trim(folder, &path, 1, true)
            {
                let at = segments[..=index].join("/");

                ctx.report(Span::new(0, 0), format!("folder {at}/ repeats \"{}\" from its path; name it \"{}\"", trim.prefix, trim.suggestion));
            }
            path.extend(words::forms(folder));
        }

        let file = segments[segments.len() - 1];
        let (stem, extension) = file.split_at(file.find('.').unwrap_or(file.len()));

        if let Some(trim) = self.trim(stem, &path, 1, true) {
            ctx.report(Span::new(0, 0), format!("file name {file} repeats \"{}\" from its path; name it \"{}{extension}\"", trim.prefix, trim.suggestion));
        }

        let module = Module::of(ctx);
        // One entity per name: a `const` and `type` pair, overloads or merged
        // interfaces are renamed together, so each name is judged once.
        let mut judged = FxHashSet::default();
        let trims: Vec<_> = module
            .declarations
            .iter()
            .filter_map(|(identifier, exported)| {
                let name = identifier.name.as_str();

                if !judged.insert(name) {
                    return None;
                }

                let trim = if *exported { self.trim(name, &workspace, 2, false) } else { self.trim(name, &path, 1, true) };

                trim.map(|trim| (*identifier, *exported, trim))
            })
            .collect();
        let mut suggested: FxHashMap<&str, usize> = FxHashMap::default();

        for (_, _, trim) in &trims {
            *suggested.entry(trim.suggestion.as_str()).or_default() += 1;
        }

        for (identifier, exported, trim) in &trims {
            let name = identifier.name.as_str();
            let said = if *exported { "its workspace name" } else { "its path" };
            // A trim to a name already bound, or two trims to one name, would declare it twice.
            let clash = if module.taken.contains(trim.suggestion.as_str()) {
                Some("this module already declares")
            } else if suggested[trim.suggestion.as_str()] > 1 {
                Some("another declaration here also trims to")
            } else {
                None
            };
            let message = if let Some(clash) = clash {
                format!(
                    "\"{name}\" repeats \"{}\" from {said}, and {clash} \"{}\"; keep a word that tells them apart instead of \"{}\"",
                    trim.prefix, trim.suggestion, trim.prefix
                )
            } else {
                format!("\"{name}\" repeats \"{}\" from {said}; name it \"{}\"", trim.prefix, trim.suggestion)
            };

            ctx.report(identifier.span, message);
        }
    }
}

impl NoRedundantNamePrefix {
    /// `name` without the prefixes in `path` it starts with, keeping at least
    /// `keep` words. With `repeat`, trimming continues while another prefix
    /// follows, so `authSignInLimit` under `auth/src/sign-in/` becomes `limit`.
    fn trim(&self, name: &str, path: &[Vec<String>], keep: usize, repeat: bool) -> Option<Trim> {
        let words = words::split(name);
        let mut at = 0;

        while let Some(prefix) =
            path.iter().filter(|prefix| words.len() >= at + prefix.len() + keep && words::starts_with(&words, at, prefix)).max_by_key(|prefix| prefix.len())
        {
            if self.cuts_term(&words, at, prefix) {
                break;
            }
            at += prefix.len();
            if !repeat {
                break;
            }
        }

        (at > 0).then(|| Trim {
            prefix: words[..at].iter().map(|word| word.lower.as_str()).collect::<Vec<_>>().join("-"),
            suggestion: words::rest(name, &words, at),
        })
    }

    /// Whether removing `prefix` at `at` would leave part of a configured term.
    fn cuts_term(&self, words: &[Word], at: usize, prefix: &[String]) -> bool {
        self.terms.iter().any(|term| term.len() > prefix.len() && term.starts_with(prefix) && words::starts_with(words, at, term))
    }
}

/// A file's module-level declarations and every name bound at module level.
struct Module<'a> {
    declarations: Vec<(&'a BindingIdentifier<'a>, bool)>,
    taken: FxHashSet<&'a str>,
}

impl<'a> Module<'a> {
    fn of(ctx: &Context<'_, 'a>) -> Self {
        let mut module = Self { declarations: Vec::new(), taken: FxHashSet::default() };
        let mut listed = FxHashSet::default();

        for statement in &ctx.semantic.nodes().program().body {
            match statement {
                Statement::ExportDeclaration(export) => module.declare(&export.declaration, true),
                Statement::ExportDefaultDeclaration(export) => {
                    let id = match &export.declaration {
                        ExportDefaultDeclarationKind::FunctionDeclaration(function) => function.id.as_ref(),
                        ExportDefaultDeclarationKind::ClassDeclaration(class) => class.id.as_ref(),
                        ExportDefaultDeclarationKind::TSInterfaceDeclaration(interface) => Some(&interface.id),
                        _ => None,
                    };

                    module.add(id, true);
                }
                Statement::ExportNamedDeclaration(export) => {
                    listed.extend(export.specifiers.iter().filter_map(|specifier| specifier.local.identifier_name()).map(|name| name.as_str()));
                }
                Statement::ImportDeclaration(import) => {
                    for specifier in import.specifiers.iter().flatten() {
                        let local = match specifier {
                            ImportDeclarationSpecifier::ImportSpecifier(it) => &it.local,
                            ImportDeclarationSpecifier::ImportDefaultSpecifier(it) => &it.local,
                            ImportDeclarationSpecifier::ImportNamespaceSpecifier(it) => &it.local,
                        };

                        module.taken.insert(local.name.as_str());
                    }
                }
                _ => {
                    if let Some(declaration) = statement.as_declaration() {
                        module.declare(declaration, false);
                    }
                }
            }
        }

        // `export { name }` exports a declaration written without `export`.
        for (identifier, exported) in &mut module.declarations {
            *exported |= listed.contains(identifier.name.as_str());
        }

        module
    }

    fn declare(&mut self, declaration: &'a Declaration<'a>, exported: bool) {
        if let Declaration::VariableDeclaration(variables) = declaration {
            for declarator in &variables.declarations {
                for identifier in declarator.id.get_binding_identifiers() {
                    self.add(Some(identifier), exported);
                }
            }
        } else {
            self.add(declaration.id(), exported);
        }
    }

    fn add(&mut self, identifier: Option<&'a BindingIdentifier<'a>>, exported: bool) {
        if let Some(identifier) = identifier {
            self.taken.insert(identifier.name.as_str());
            self.declarations.push((identifier, exported));
        }
    }
}
