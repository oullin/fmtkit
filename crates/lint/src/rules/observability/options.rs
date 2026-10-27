//! Strict configuration shared by the two catch rules.

use globset::{Glob, GlobSet, GlobSetBuilder};
use oxc_ast::ast::Expression;
use oxc_syntax::identifier::is_identifier_name;
use serde_json::Value;

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
