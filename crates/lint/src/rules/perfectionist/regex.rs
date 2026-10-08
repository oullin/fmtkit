//! Perfectionist's `RegexOption`: ECMAScript patterns, matched like `RegExp#test`.

use regress::Regex;
use serde_json::{Map, Value};

/// `new RegExp(pattern, flags)`.
#[derive(Debug)]
pub struct JsRegex {
    regex: Regex,
    sticky: bool,
    unicode: bool,
}

impl JsRegex {
    pub fn new(pattern: &str, flags: &str) -> Result<Self, String> {
        let mut seen = String::new();

        for flag in flags.chars() {
            if !"dgimsuvy".contains(flag) || seen.contains(flag) {
                return Err(format!("invalid regular expression flags \"{flags}\""));
            }

            seen.push(flag);
        }

        let unicode = seen.contains('u') || seen.contains('v');

        if seen.contains('u') && seen.contains('v') {
            return Err(format!("invalid regular expression flags \"{flags}\""));
        }

        let engine: String = seen.chars().filter(|flag| "imsuv".contains(*flag)).collect();
        let regex = Regex::with_flags(pattern, engine.as_str()).map_err(|e| format!("invalid regular expression /{pattern}/{flags}: {e}"))?;

        Ok(Self { regex, sticky: seen.contains('y'), unicode })
    }

    /// `RegExp#test` on a fresh regex: UTF-16 code units, `y` anchors at 0.
    pub fn test(&self, text: &str) -> bool {
        let units: Vec<u16> = text.encode_utf16().collect();
        let found = if self.unicode { self.regex.find_from_utf16(&units, 0).next() } else { self.regex.find_from_ucs2(&units, 0).next() };

        found.is_some_and(|found| !self.sticky || found.start() == 0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Shallow,
    Deep,
}

#[derive(Debug)]
pub struct Pattern {
    pub regex: JsRegex,
    pub scope: Scope,
}

/// One pattern or several; matching any of them matches.
#[derive(Debug)]
pub struct RegexOption {
    pub patterns: Vec<Pattern>,
}

impl RegexOption {
    /// `string | {pattern, flags?} | (string | {pattern, flags?})[]`, plus
    /// `scope` on the objects when `scoped`.
    pub fn parse(value: &Value, scoped: bool, key: &str) -> Result<Self, String> {
        let patterns = match value {
            Value::Array(items) => items.iter().map(|item| single(item, scoped, key)).collect::<Result<_, _>>()?,
            other => vec![single(other, scoped, key)?],
        };

        Ok(Self { patterns })
    }

    pub fn matches(&self, text: &str) -> bool {
        self.patterns.iter().any(|pattern| pattern.regex.test(text))
    }

    /// Perfectionist's `matchesScopedExpressions`: shallow patterns look at the
    /// closest parent only, deep ones at every parent; both only at parents
    /// `allowed` keeps.
    pub fn matches_scoped<T>(&self, parents: &[T], allowed: impl Fn(&T) -> bool, values: impl Fn(&T) -> Vec<String>) -> bool {
        let matches_parent = |parent: &T, scope: Scope| {
            let values = values(parent);

            self.patterns.iter().filter(|p| p.scope == scope).any(|p| values.iter().any(|value| p.regex.test(value)))
        };
        let shallow = parents.first().is_some_and(|first| allowed(first) && matches_parent(first, Scope::Shallow));

        shallow || parents.iter().filter(|parent| allowed(parent)).any(|parent| matches_parent(parent, Scope::Deep))
    }
}

fn single(value: &Value, scoped: bool, key: &str) -> Result<Pattern, String> {
    match value {
        Value::String(pattern) => Ok(Pattern { regex: JsRegex::new(pattern, "")?, scope: Scope::Shallow }),
        Value::Object(object) => object_pattern(object, scoped, key),
        _ => Err(format!("\"{key}\" must be a regular expression string, a {{ pattern, flags }} object, or an array of them")),
    }
}

fn object_pattern(object: &Map<String, Value>, scoped: bool, key: &str) -> Result<Pattern, String> {
    let mut pattern = None;
    let mut flags = "";
    let mut scope = Scope::Shallow;

    for (name, value) in object {
        match (name.as_str(), value) {
            ("pattern", Value::String(value)) => pattern = Some(value.as_str()),
            ("flags", Value::String(value)) => flags = value,
            ("scope", Value::String(value)) if scoped => {
                scope = match value.as_str() {
                    "shallow" => Scope::Shallow,
                    "deep" => Scope::Deep,
                    other => return Err(format!("\"{key}.scope\" must be \"shallow\" or \"deep\", not \"{other}\"")),
                };
            }
            ("source", _) => {
                return Err("Invalid configuration: please enter your RegExp expressions as strings.\nFor example, write \".*foo\" instead of /.*foo/".into());
            }
            (name, _) => return Err(format!("\"{key}\": unexpected or invalid \"{name}\"")),
        }
    }

    let pattern = pattern.ok_or_else(|| format!("\"{key}\" objects need a \"pattern\""))?;

    Ok(Pattern { regex: JsRegex::new(pattern, flags)?, scope })
}
