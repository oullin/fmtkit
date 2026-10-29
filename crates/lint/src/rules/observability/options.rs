//! Strict configuration for the catch rules.

use globset::{Glob, GlobSet, GlobSetBuilder};
use oxc_ast::ast::Expression;
use oxc_syntax::identifier::is_identifier_name;
use serde_json::Value;

use super::super::boundaries::Paths;
use super::shared::{member, unwrap};

const DEFAULT_OBSERVERS: &[&str] = &["reportException", "*.reportException", "OperationalLogger.*", "console.warn", "console.error", "Sentry.captureException"];

pub struct Options {
    pub ignore: GlobSet,
    pub observers: Vec<Observer>,
}

impl Options {
    pub fn parse(options: &[Value], allow_observers: bool) -> Result<Self, String> {
        let mut ignore = GlobSetBuilder::new();
        let mut observers = DEFAULT_OBSERVERS.iter().map(|name| Observer::parse(name)).collect::<Result<_, _>>()?;
        let object = match options {
            [] => None,
            [Value::Object(object)] => Some(object),
            [_] => return Err("expected an options object".into()),
            _ => return Err("expected at most one options object".into()),
        };

        for (key, value) in object.into_iter().flatten() {
            match key.as_str() {
                "ignore" => {
                    for pattern in strings(key, value)? {
                        ignore.add(Glob::new(pattern).map_err(|error| format!("invalid ignore glob {pattern:?}: {error}"))?);
                    }
                }
                "observers" if allow_observers => {
                    observers = strings(key, value)?.into_iter().map(Observer::parse).collect::<Result<_, _>>()?;
                }
                other => return Err(format!("unknown option {other:?}")),
            }
        }

        Ok(Self { ignore: ignore.build().map_err(|error| format!("invalid ignore globs: {error}"))?, observers })
    }
}

fn strings<'v>(key: &str, value: &'v Value) -> Result<Vec<&'v str>, String> {
    let invalid = || format!("{key:?} must be an array of unique strings");
    let mut strings = Vec::new();

    for item in value.as_array().ok_or_else(invalid)? {
        let item = item.as_str().ok_or_else(invalid)?;

        if strings.contains(&item) {
            return Err(invalid());
        }

        strings.push(item);
    }

    Ok(strings)
}

pub enum Observer {
    Bare(String),
    Member { owner: String, name: String },
}

impl Observer {
    fn parse(pattern: &str) -> Result<Self, String> {
        if is_identifier_name(pattern) {
            return Ok(Self::Bare(pattern.to_owned()));
        }

        if let Some((owner, name)) = pattern.split_once('.')
            && ((is_identifier_name(owner) && (is_identifier_name(name) || name == "*")) || (owner == "*" && is_identifier_name(name)))
        {
            return Ok(Self::Member { owner: owner.to_owned(), name: name.to_owned() });
        }

        Err(format!("invalid observer {pattern:?}: expected name, Owner.member, Owner.*, or *.member"))
    }

    pub fn matches(&self, callee: &Expression<'_>) -> bool {
        match self {
            Self::Bare(name) => matches!(unwrap(callee), Expression::Identifier(id) if id.name == name.as_str()),
            Self::Member { owner, name } => member(callee).is_some_and(|(object, property, _)| {
                (name == "*" || name == property) && (owner == "*" || matches!(unwrap(object), Expression::Identifier(id) if id.name == owner.as_str()))
            }),
        }
    }
}

pub struct BoundaryOptions {
    pub allow: Paths,
    pub translate: bool,
    pub boundaries: Option<Boundaries>,
}

impl BoundaryOptions {
    pub fn parse(values: &[Value]) -> Result<Self, String> {
        let object = match values {
            [] => None,
            [Value::Object(object)] => Some(object),
            [_] => return Err("expected an options object".into()),
            _ => return Err("expected at most one options object".into()),
        };
        let mut options = Self { allow: Paths::default(), translate: true, boundaries: None };

        for (key, value) in object.into_iter().flatten() {
            match key.as_str() {
                "allow" => {
                    let patterns = value
                        .as_array()
                        .ok_or("allow must be an array of strings")?
                        .iter()
                        .map(|item| item.as_str().map(str::to_owned).ok_or("allow must be an array of strings"))
                        .collect::<Result<Vec<_>, _>>()?;

                    options.allow = Paths::parse(key, &patterns)?;
                }
                "translate" => options.translate = value.as_bool().ok_or("translate must be a boolean")?,
                "boundaries" => options.boundaries = Some(Boundaries::parse(value)?),
                _ => return Err(format!("unknown option {key:?}")),
            }
        }

        Ok(options)
    }
}

/// Files that import a failure boundary are boundaries themselves: a static,
/// non-type import of one of `names` from one of `modules`, matched exactly.
pub struct Boundaries {
    pub modules: Vec<String>,
    pub names: Vec<String>,
}

impl Boundaries {
    fn parse(value: &Value) -> Result<Self, String> {
        let object = value.as_object().ok_or("boundaries must be an object")?;
        let mut modules = None;
        let mut names = None;

        for (key, value) in object {
            match key.as_str() {
                "modules" => modules = Some(non_blank_strings("boundaries.modules", value)?),
                "names" => {
                    let parsed = non_blank_strings("boundaries.names", value)?;

                    if let Some(name) = parsed.iter().find(|name| !is_identifier_name(name)) {
                        return Err(format!("invalid boundaries name {name:?}: expected an identifier name"));
                    }

                    names = Some(parsed);
                }
                _ => return Err(format!("unknown boundaries option {key:?}")),
            }
        }

        Ok(Self { modules: modules.ok_or("boundaries.modules is required")?, names: names.ok_or("boundaries.names is required")? })
    }

    /// Whether one of the file's named value imports is a configured boundary.
    pub fn admits<'i>(&self, imports: impl IntoIterator<Item = (&'i str, &'i str)>) -> bool {
        imports.into_iter().any(|(module, name)| self.modules.iter().any(|m| m == module) && self.names.iter().any(|n| n == name))
    }
}

fn non_blank_strings(key: &str, value: &Value) -> Result<Vec<String>, String> {
    let invalid = || format!("{key} must be a non-empty array of unique non-blank strings");
    let mut strings: Vec<String> = Vec::new();

    for item in value.as_array().ok_or_else(invalid)? {
        let item = item.as_str().ok_or_else(invalid)?;

        if item.trim().is_empty() || strings.iter().any(|known| known == item) {
            return Err(invalid());
        }

        strings.push(item.to_owned());
    }

    if strings.is_empty() {
        return Err(invalid());
    }

    Ok(strings)
}
