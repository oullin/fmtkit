//! Strict boundary options and ordered repository-relative exemptions.

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use serde_json::{Map, Value};

use super::Kind;

#[derive(Default)]
pub struct Options {
    pub exempt: Paths,
    pub names: Vec<String>,
    pub rules: Vec<Specifier>,
}

impl Options {
    pub fn parse(values: &[Value], kind: Kind) -> Result<Self, String> {
        let object = match values {
            [] => None,
            [Value::Object(object)] => Some(object),
            [_] => return Err("expected an options object".into()),
            _ => return Err("expected at most one options object".into()),
        };
        let mut options = Self::default();

        for (key, value) in object.into_iter().flatten() {
            match key.as_str() {
                "exempt" => options.exempt = Paths::parse(key, &strings(key, value)?)?,
                "names" if kind == Kind::ReservedInternalName => options.names = strings(key, value)?,
                "rules" if kind == Kind::ReservedSpecifier => options.rules = specifiers(value)?,
                _ => return Err(format!("unknown option {key:?}")),
            }
        }

        if kind == Kind::ReservedInternalName && options.names.is_empty() {
            return Err("names is required".into());
        }
        if kind == Kind::ReservedSpecifier && options.rules.is_empty() {
            return Err("rules must contain at least one specifier rule".into());
        }

        Ok(options)
    }
}

fn strings(key: &str, value: &Value) -> Result<Vec<String>, String> {
    value
        .as_array()
        .ok_or_else(|| format!("{key} must be an array of strings"))?
        .iter()
        .map(|item| item.as_str().map(str::to_owned).ok_or_else(|| format!("{key} must be an array of strings")))
        .collect()
}

#[derive(Default)]
pub struct Paths {
    globs: GlobSet,
    excluded: Vec<bool>,
}

impl Paths {
    fn parse(key: &str, patterns: &[String]) -> Result<Self, String> {
        let mut globs = GlobSetBuilder::new();
        let mut excluded = Vec::new();

        for pattern in patterns {
            let (include, glob) = pattern.strip_prefix('!').map_or((false, pattern.as_str()), |glob| (true, glob));

            if glob.trim().is_empty() {
                return Err(format!("{key}: a pattern is blank"));
            }
            if glob.contains("{package}") {
                return Err(format!("{key}: {{package}} is not supported; expand it to repository-relative package directories"));
            }

            // Cloud uses path.Match per segment, where braces are literal and
            // a single star cannot cross a directory separator.
            let glob = glob.replace('{', "\\{").replace('}', "\\}");
            let built = GlobBuilder::new(&glob)
                .literal_separator(true)
                .backslash_escape(true)
                .build()
                .map_err(|error| format!("invalid {key} glob {pattern:?}: {error}"))?;

            globs.add(built);
            excluded.push(!include);
        }

        Ok(Self { globs: globs.build().map_err(|error| format!("invalid {key} globs: {error}"))?, excluded })
    }

    pub fn matches(&self, path: &str) -> bool {
        self.globs.matches(path).last().is_some_and(|&index| self.excluded[index])
    }
}

pub struct Specifier {
    pub name: String,
    pub match_modules: Vec<String>,
    pub except: Vec<String>,
    pub allow: Paths,
    pub message: String,
}

fn required(object: &Map<String, Value>, key: &str) -> Result<String, String> {
    object.get(key).and_then(Value::as_str).filter(|value| !value.trim().is_empty()).map(str::to_owned).ok_or_else(|| format!("{key} is required"))
}

fn specifiers(value: &Value) -> Result<Vec<Specifier>, String> {
    let mut rules: Vec<Specifier> = Vec::new();

    for value in value.as_array().ok_or("rules must be an array of objects")? {
        let object = value.as_object().ok_or("rules must be an array of objects")?;

        for key in object.keys() {
            if !["name", "match", "except", "allow", "message"].contains(&key.as_str()) {
                return Err(format!("unknown specifier option {key:?}"));
            }
        }

        let name = required(object, "name")?;

        if super::RULES.iter().any(|(rule, _)| rule.strip_prefix("boundaries/") == Some(name.as_str()))
            || ["registered-workspace", "undeclared-workspace-dependency"].contains(&name.as_str())
        {
            return Err(format!("{name:?} is a built-in rule's name"));
        }
        if rules.iter().any(|rule| rule.name == name) {
            return Err(format!("{name:?} is named twice"));
        }

        let read = |key: &str| -> Result<Vec<String>, String> {
            let values = object.get(key).map_or_else(|| Ok(Vec::new()), |value| strings(key, value))?;

            if values.iter().any(|value| value.strip_prefix('!').unwrap_or(value).trim().is_empty()) {
                return Err(format!("{key}: an entry is blank"));
            }

            Ok(values)
        };
        let match_modules = read("match")?;

        if match_modules.is_empty() {
            return Err("match is required".into());
        }

        rules.push(Specifier {
            name,
            match_modules,
            except: read("except")?,
            allow: Paths::parse("allow", &read("allow")?)?,
            message: required(object, "message")?,
        });
    }

    Ok(rules)
}

pub fn matches(modules: &[String], specifier: &str) -> bool {
    modules.iter().any(|module| specifier == module || specifier.strip_prefix(module).is_some_and(|suffix| suffix.starts_with('/')))
}
