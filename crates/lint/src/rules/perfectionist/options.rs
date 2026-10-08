//! Perfectionist's options: the JSON schema each rule accepts, `complete()`
//! (defaults, then settings, then the option), the validations every rule
//! runs, and the group helpers built on them.

use std::sync::Arc;

use icu_collator::CollatorBorrowed;
use icu_collator::options::CollatorOptions;
use icu_locale_core::Locale;
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::{Map, Value};

use super::estree::Selector;
use super::regex::RegexOption;
use super::source::{Comment, directive, js_trim};

/// What differs between the rules' schemas.
pub struct Schema {
    pub selectors: &'static [&'static str],
    pub modifiers: &'static [&'static str],
    /// Custom group match keys besides `elementNamePattern`.
    pub match_keys: &'static [&'static str],
    /// Whether `sortBy` is a sort option (object types, interfaces, objects).
    pub sort_by: bool,
    pub partition_by_comment: bool,
    /// `useConfigurationIf` keys besides `allNamesMatchPattern`.
    pub when_keys: &'static [&'static str],
    /// Rule-specific top-level keys.
    pub extra_keys: &'static [&'static str],
    pub defaults: fn() -> Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Alphabetical,
    Natural,
    LineLength,
    Custom,
    Unsorted,
    SubgroupOrder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum By {
    Name,
    Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Special {
    Remove,
    Trim,
    Keep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortByValue {
    Always,
    IfNumericEnum,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    Destructured,
    NonDestructured,
}

/// One comparator's settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sort {
    pub kind: Kind,
    pub order: Order,
    pub by: By,
}

/// Sort settings that override another's where present.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SortPatch {
    pub kind: Option<Kind>,
    pub order: Option<Order>,
    pub by: Option<By>,
}

impl SortPatch {
    fn apply(self, sort: Sort) -> Sort {
        Sort { kind: self.kind.unwrap_or(sort.kind), order: self.order.unwrap_or(sort.order), by: self.by.unwrap_or(sort.by) }
    }

    /// `{...self, ...other}`.
    fn merge(self, other: Self) -> Self {
        Self { kind: other.kind.or(self.kind), order: other.order.or(self.order), by: other.by.or(self.by) }
    }
}

/// The main and fallback comparators' settings for one group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spec {
    pub sort: Sort,
    pub fallback: SortPatch,
}

impl Spec {
    pub fn fallback_sort(self) -> Sort {
        self.fallback.apply(self.sort)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Newlines {
    Ignore,
    Count(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inside {
    Ignore,
    Count(u32),
    Between,
}

/// `string | string[]` in a group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Names {
    One(String),
    Many(Vec<String>),
}

impl Names {
    fn contains(&self, group: &str) -> bool {
        match self {
            Self::One(name) => name == group,
            Self::Many(names) => names.iter().any(|name| name == group),
        }
    }

    fn all(&self) -> Vec<&str> {
        match self {
            Self::One(name) => vec![name.as_str()],
            Self::Many(names) => names.iter().map(String::as_str).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Group {
    Names(Names),
    Newlines(Newlines),
    Override { names: Names, sort: SortPatch, fallback: SortPatch, newlines_inside: Option<Newlines> },
}

impl Group {
    fn names(&self) -> Option<&Names> {
        match self {
            Self::Names(names) | Self::Override { names, .. } => Some(names),
            Self::Newlines(_) => None,
        }
    }

    /// `computeGroupName`.
    fn name(&self) -> Option<&str> {
        match self.names() {
            Some(Names::One(name)) => Some(name),
            _ => None,
        }
    }
}

/// One way a custom group can match an element.
#[derive(Debug, Default)]
pub struct Matcher {
    pub name: Option<RegexOption>,
    pub value: Option<RegexOption>,
    pub selector: Option<String>,
    pub modifiers: Option<Vec<String>>,
}

impl Matcher {
    /// `doesSingleCustomGroupMatch`.
    pub fn matches(&self, name: &str, value: Option<&str>, selectors: &[&str], modifiers: &[&str]) -> bool {
        if self.selector.as_deref().is_some_and(|selector| !selectors.contains(&selector)) {
            return false;
        }

        if self.modifiers.as_ref().is_some_and(|wanted| wanted.iter().any(|modifier| !modifiers.contains(&modifier.as_str()))) {
            return false;
        }

        if self.name.as_ref().is_some_and(|pattern| !pattern.matches(name)) {
            return false;
        }

        self.value.as_ref().is_none_or(|pattern| pattern.matches(value.unwrap_or_default()))
    }
}

#[derive(Debug)]
pub struct CustomGroup {
    pub name: String,
    /// `anyOf`, or the group's own match options.
    pub matchers: Vec<Matcher>,
    pub sort: SortPatch,
    pub fallback: SortPatch,
    pub newlines_inside: Option<Newlines>,
}

/// `boolean | RegexOption` inside `partitionByComment`.
#[derive(Debug)]
pub enum CommentMatch {
    Bool(bool),
    Regex(RegexOption),
}

impl CommentMatch {
    fn matches(&self, trimmed: &str) -> bool {
        match self {
            Self::Bool(value) => *value,
            Self::Regex(regex) => regex.matches(trimmed),
        }
    }
}

#[derive(Debug)]
pub enum PartitionComment {
    Off,
    Any(CommentMatch),
    Split { block: Option<CommentMatch>, line: Option<CommentMatch> },
}

impl PartitionComment {
    pub fn is_on(&self) -> bool {
        !matches!(self, Self::Off)
    }

    /// `isPartitionComment`.
    pub fn matches(&self, comment: &Comment, value: &str) -> bool {
        if !self.is_on() || directive(value).is_some() {
            return false;
        }

        let trimmed = js_trim(value);

        match self {
            Self::Off => false,
            Self::Any(matcher) => matcher.matches(trimmed),
            Self::Split { block, line } => {
                let relevant = if comment.block { block } else { line };

                relevant.as_ref().is_some_and(|matcher| matcher.matches(trimmed))
            }
        }
    }
}

/// `useConfigurationIf`.
#[derive(Debug, Default)]
pub struct When {
    pub all_names: Option<RegexOption>,
    pub selector: Option<Selector>,
    pub tag: Option<RegexOption>,
    pub numeric_keys_only: Option<bool>,
    pub declaration_comment: Option<RegexOption>,
    pub declaration: Option<RegexOption>,
    pub calling_function: Option<RegexOption>,
    pub object_type: Option<ObjectType>,
}

impl When {
    /// `passesAllNamesMatchPatternFilter`.
    pub fn all_names_match<S: AsRef<str>>(&self, names: &[S]) -> bool {
        self.all_names.as_ref().is_none_or(|pattern| names.iter().all(|name| pattern.matches(name.as_ref())))
    }
}

/// The settings every comparator shares.
pub struct Text {
    pub special: Special,
    pub ignore_case: bool,
    /// Code point to index in `alphabet`; the last occurrence wins.
    pub alphabet: FxHashMap<u32, usize>,
    pub collator: CollatorBorrowed<'static>,
}

impl std::fmt::Debug for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Text").field("special", &self.special).field("ignore_case", &self.ignore_case).finish_non_exhaustive()
    }
}

/// One rule's complete options.
#[derive(Debug)]
#[allow(clippy::struct_excessive_bools, reason = "mirrors perfectionist's option schema")]
pub struct Options {
    pub sort: Sort,
    pub fallback: SortPatch,
    pub text: Arc<Text>,
    pub partition_comment: PartitionComment,
    pub partition_newline: bool,
    pub newlines_between: Newlines,
    pub newlines_inside: Inside,
    pub groups: Vec<Group>,
    pub custom_groups: Vec<CustomGroup>,
    /// Every group name in `groups`.
    group_names: FxHashSet<String>,
    pub when: When,
    pub sort_by_value: SortByValue,
    pub experimental: bool,
    pub ignore_callback: Option<RegexOption>,
    pub partition_by_computed_key: bool,
    pub styled_components: bool,
}

impl Options {
    /// `computeGroup`: the first custom group that matches and is used, else
    /// the first used predefined group, else `unknown`.
    pub fn compute_group(&self, predefined: &[String], matches: impl Fn(&Matcher) -> bool) -> String {
        for custom in &self.custom_groups {
            if self.group_names.contains(&custom.name) && custom.matchers.iter().any(&matches) {
                return custom.name.clone();
            }
        }

        predefined.iter().find(|group| self.group_names.contains(*group)).cloned().unwrap_or_else(|| "unknown".to_owned())
    }

    /// `getGroupIndex`.
    pub fn group_index(&self, group: &str) -> usize {
        self.groups.iter().position(|candidate| candidate.names().is_some_and(|names| names.contains(group))).unwrap_or(self.groups.len())
    }

    /// `computeOverriddenOptionsByGroupIndex`.
    pub fn spec(&self, group_index: usize) -> Spec {
        let mut sort = self.sort;
        let mut fallback = self.fallback;
        let group = self.groups.get(group_index);

        if let Some(Group::Override { sort: patch, fallback: extra, .. }) = group {
            sort = patch.apply(sort);
            fallback = fallback.merge(*extra);
        }

        if let Some(custom) = group.and_then(Group::name).and_then(|name| self.custom_groups.iter().find(|custom| custom.name == name)) {
            sort = custom.sort.apply(sort);
            fallback = fallback.merge(custom.fallback);
        }

        Spec { sort, fallback }
    }

    /// The array group holding `group`, for `subgroup-order`.
    pub fn subgroup(&self, group: &str) -> Option<usize> {
        self.groups.iter().position(|candidate| matches!(candidate.names(), Some(names @ Names::Many(_)) if names.contains(group)))
    }

    pub fn subgroup_position(&self, subgroup: usize, group: &str) -> usize {
        match self.groups[subgroup].names() {
            Some(Names::Many(names)) => names.iter().position(|name| name == group).unwrap_or(0),
            _ => 0,
        }
    }

    /// `getNewlinesBetweenOption`.
    pub fn newlines(&self, left: usize, right: usize) -> Newlines {
        if left == right {
            return self.newlines_inside(left);
        }

        if right == left + 2 {
            if let Some(Group::Newlines(newlines)) = self.groups.get(left + 1) {
                return *newlines;
            }

            return self.newlines_between;
        }

        if right < left + 2 {
            return self.newlines_between;
        }

        let relevant = self.groups.get(left..=right).unwrap_or_default();
        let mut values = Vec::new();

        for (i, group) in relevant.iter().enumerate() {
            match group {
                Group::Newlines(newlines) => values.push(*newlines),
                _ if i > 0 && !matches!(relevant[i - 1], Group::Newlines(_)) => values.push(self.newlines_between),
                _ => {}
            }
        }

        let max = values.iter().filter_map(|value| if let Newlines::Count(n) = value { Some(*n) } else { None }).max();

        match max {
            Some(n) if n >= 1 => Newlines::Count(n),
            _ if values.contains(&Newlines::Ignore) => Newlines::Ignore,
            _ => Newlines::Count(0),
        }
    }

    fn newlines_inside(&self, index: usize) -> Newlines {
        let global = match self.newlines_inside {
            Inside::Between if self.newlines_between == Newlines::Ignore => Newlines::Ignore,
            Inside::Between => Newlines::Count(0),
            Inside::Ignore => Newlines::Ignore,
            Inside::Count(n) => Newlines::Count(n),
        };
        let Some(group) = self.groups.get(index) else { return global };
        let custom = group.name().and_then(|name| self.custom_groups.iter().find(|custom| custom.name == name)).and_then(|custom| custom.newlines_inside);
        let overridden = if let Group::Override { newlines_inside, .. } = group { *newlines_inside } else { None };

        custom.or(overridden).unwrap_or(global)
    }
}

/// `generatePredefinedGroups`: every modifier permutation before each
/// selector, longest first, then the bare selector.
pub fn predefined_groups(selectors: &[&str], modifiers: &[&str]) -> Vec<String> {
    let mut permutations = Vec::new();

    for size in (1..=modifiers.len()).rev() {
        for combination in combinations(modifiers, size) {
            permute(&mut combination.clone(), 0, &mut permutations);
        }
    }

    let mut groups = Vec::new();

    for selector in selectors {
        for permutation in &permutations {
            groups.push(format!("{}-{selector}", permutation.join("-")));
        }

        groups.push((*selector).to_owned());
    }

    groups
}

fn combinations<'s>(items: &[&'s str], size: usize) -> Vec<Vec<&'s str>> {
    fn backtrack<'s>(items: &[&'s str], size: usize, start: usize, current: &mut Vec<&'s str>, out: &mut Vec<Vec<&'s str>>) {
        if current.len() == size {
            out.push(current.clone());
            return;
        }

        for i in start..items.len() {
            current.push(items[i]);
            backtrack(items, size, i + 1, current, out);
            current.pop();
        }
    }

    let mut out = Vec::new();

    backtrack(items, size, 0, &mut Vec::new(), &mut out);
    out
}

fn permute<'s>(items: &mut Vec<&'s str>, first: usize, out: &mut Vec<Vec<&'s str>>) {
    if first == items.len() {
        out.push(items.clone());
        return;
    }

    for i in first..items.len() {
        items.swap(first, i);
        permute(items, first + 1, out);
        items.swap(first, i);
    }
}

/// Builds the options for one `context.options` entry (or none), after
/// checking it against `schema` and running perfectionist's validations.
pub fn build(schema: &Schema, option: Option<&Value>, settings: Option<&Value>) -> Result<Options, String> {
    let mut merged = match (schema.defaults)() {
        Value::Object(map) => map,
        _ => Map::new(),
    };

    if let Some(settings) = settings {
        let settings = settings.as_object().ok_or("perfectionist settings must be an object")?;

        check_settings(settings)?;
        merged.extend(settings.iter().map(|(k, v)| (k.clone(), v.clone())));
    }

    if let Some(option) = option {
        let option = option.as_object().ok_or("options must be objects")?;

        check_keys(schema, option)?;
        merged.extend(option.iter().map(|(k, v)| (k.clone(), v.clone())));
    }

    let options = parse(schema, &merged)?;

    validate(schema, &options)?;
    Ok(options)
}

/// `getSettings`.
fn check_settings(settings: &Map<String, Value>) -> Result<(), String> {
    const ALLOWED: [&str; 12] = [
        "partitionByComment",
        "partitionByNewLine",
        "specialCharacters",
        "newlinesBetween",
        "newlinesInside",
        "fallbackSort",
        "ignoreCase",
        "tsconfig",
        "alphabet",
        "locales",
        "order",
        "type",
    ];

    let invalid: Vec<&str> = settings.keys().map(String::as_str).filter(|key| !ALLOWED.contains(key)).collect();

    if invalid.is_empty() { Ok(()) } else { Err(format!("Invalid Perfectionist setting(s): {}", invalid.join(", "))) }
}

const COMMON_KEYS: [&str; 13] = [
    "fallbackSort",
    "type",
    "specialCharacters",
    "ignoreCase",
    "alphabet",
    "locales",
    "order",
    "customGroups",
    "newlinesInside",
    "groups",
    "newlinesBetween",
    "useConfigurationIf",
    "partitionByNewLine",
];

fn check_keys(schema: &Schema, option: &Map<String, Value>) -> Result<(), String> {
    for key in option.keys() {
        let known = COMMON_KEYS.contains(&key.as_str())
            || (schema.sort_by && key == "sortBy")
            || (schema.partition_by_comment && key == "partitionByComment")
            || schema.extra_keys.contains(&key.as_str());

        if !known {
            return Err(format!("unknown option \"{key}\""));
        }
    }

    Ok(())
}

fn parse(schema: &Schema, map: &Map<String, Value>) -> Result<Options, String> {
    let get = |key: &str| map.get(key);
    let sort = Sort {
        kind: kind(get("type").ok_or("missing \"type\"")?, "type")?,
        order: order(get("order").ok_or("missing \"order\"")?, "order")?,
        by: if schema.sort_by { by(get("sortBy").ok_or("missing \"sortBy\"")?, "sortBy")? } else { By::Name },
    };
    let fallback = match get("fallbackSort") {
        Some(value) => fallback(value, schema.sort_by, "fallbackSort")?,
        None => SortPatch { kind: Some(Kind::Unsorted), ..SortPatch::default() },
    };
    let text = Text {
        special: special(get("specialCharacters").unwrap_or(&Value::Null))?,
        ignore_case: boolean(get("ignoreCase"), "ignoreCase")?.unwrap_or(true),
        alphabet: alphabet(get("alphabet"))?,
        collator: collator(get("locales"))?,
    };
    let partition_comment = if schema.partition_by_comment { partition_comment(get("partitionByComment"))? } else { PartitionComment::Off };
    let groups = groups(get("groups"), schema.sort_by)?;
    let group_names = groups.iter().filter_map(Group::names).flat_map(Names::all).map(str::to_owned).collect();

    Ok(Options {
        sort,
        fallback,
        text: Arc::new(text),
        partition_comment,
        partition_newline: boolean(get("partitionByNewLine"), "partitionByNewLine")?.unwrap_or(false),
        newlines_between: match get("newlinesBetween") {
            Some(value) => newlines(value, "newlinesBetween")?,
            None => Newlines::Ignore,
        },
        newlines_inside: match get("newlinesInside") {
            Some(Value::String(value)) if value == "newlinesBetween" => Inside::Between,
            Some(value) => match newlines(value, "newlinesInside")? {
                Newlines::Ignore => Inside::Ignore,
                Newlines::Count(n) => Inside::Count(n),
            },
            None => Inside::Between,
        },
        groups,
        custom_groups: custom_groups(schema, get("customGroups"))?,
        group_names,
        when: when(schema, get("useConfigurationIf"))?,
        sort_by_value: match get("sortByValue").and_then(Value::as_str) {
            None | Some("ifNumericEnum") => SortByValue::IfNumericEnum,
            Some("always") => SortByValue::Always,
            Some("never") => SortByValue::Never,
            Some(other) => return Err(format!("\"sortByValue\" must be \"always\", \"ifNumericEnum\" or \"never\", not \"{other}\"")),
        },
        experimental: boolean(get("useExperimentalDependencyDetection"), "useExperimentalDependencyDetection")?.unwrap_or(true),
        ignore_callback: get("ignoreCallbackDependenciesPatterns")
            .map(|value| RegexOption::parse(value, false, "ignoreCallbackDependenciesPatterns"))
            .transpose()?,
        partition_by_computed_key: boolean(get("partitionByComputedKey"), "partitionByComputedKey")?.unwrap_or(false),
        styled_components: boolean(get("styledComponents"), "styledComponents")?.unwrap_or(true),
    })
}

fn kind(value: &Value, key: &str) -> Result<Kind, String> {
    Ok(match value.as_str() {
        Some("alphabetical") => Kind::Alphabetical,
        Some("natural") => Kind::Natural,
        Some("line-length") => Kind::LineLength,
        Some("custom") => Kind::Custom,
        Some("unsorted") => Kind::Unsorted,
        Some("subgroup-order") => Kind::SubgroupOrder,
        _ => return Err(format!("\"{key}\" must be one of alphabetical, natural, line-length, custom, unsorted, subgroup-order")),
    })
}

fn order(value: &Value, key: &str) -> Result<Order, String> {
    match value.as_str() {
        Some("asc") => Ok(Order::Asc),
        Some("desc") => Ok(Order::Desc),
        _ => Err(format!("\"{key}\" must be \"asc\" or \"desc\"")),
    }
}

fn by(value: &Value, key: &str) -> Result<By, String> {
    match value.as_str() {
        Some("name") => Ok(By::Name),
        Some("value") => Ok(By::Value),
        _ => Err(format!("\"{key}\" must be \"name\" or \"value\"")),
    }
}

fn special(value: &Value) -> Result<Special, String> {
    match value {
        Value::Null => Ok(Special::Keep),
        Value::String(text) if text == "keep" => Ok(Special::Keep),
        Value::String(text) if text == "trim" => Ok(Special::Trim),
        Value::String(text) if text == "remove" => Ok(Special::Remove),
        _ => Err("\"specialCharacters\" must be \"remove\", \"trim\" or \"keep\"".into()),
    }
}

fn boolean(value: Option<&Value>, key: &str) -> Result<Option<bool>, String> {
    match value {
        None => Ok(None),
        Some(Value::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(format!("\"{key}\" must be a boolean")),
    }
}

fn string<'v>(value: &'v Value, key: &str) -> Result<&'v str, String> {
    value.as_str().ok_or_else(|| format!("\"{key}\" must be a string"))
}

fn object<'v>(value: &'v Value, key: &str) -> Result<&'v Map<String, Value>, String> {
    value.as_object().ok_or_else(|| format!("\"{key}\" must be an object"))
}

fn array<'v>(value: &'v Value, key: &str) -> Result<&'v [Value], String> {
    value.as_array().map(Vec::as_slice).ok_or_else(|| format!("\"{key}\" must be an array"))
}

fn newlines(value: &Value, key: &str) -> Result<Newlines, String> {
    match value {
        Value::String(text) if text == "ignore" => Ok(Newlines::Ignore),
        Value::Number(number) => number
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .map(Newlines::Count)
            .ok_or_else(|| format!("\"{key}\" must be \"ignore\" or a non-negative integer")),
        _ => Err(format!("\"{key}\" must be \"ignore\" or a non-negative integer")),
    }
}

/// `{type, order?, sortBy?}`.
fn fallback(value: &Value, sort_by: bool, key: &str) -> Result<SortPatch, String> {
    let map = object(value, key)?;
    let mut patch = SortPatch::default();

    for (name, value) in map {
        match name.as_str() {
            "type" => patch.kind = Some(kind(value, &format!("{key}.type"))?),
            "order" => patch.order = Some(order(value, &format!("{key}.order"))?),
            "sortBy" if sort_by => patch.by = Some(by(value, &format!("{key}.sortBy"))?),
            other => return Err(format!("\"{key}\": unexpected \"{other}\"")),
        }
    }

    if patch.kind.is_none() {
        return Err(format!("\"{key}\" needs a \"type\""));
    }

    Ok(patch)
}

fn alphabet(value: Option<&Value>) -> Result<FxHashMap<u32, usize>, String> {
    let text = match value {
        None => "",
        Some(value) => string(value, "alphabet")?,
    };

    Ok(text.chars().enumerate().map(|(index, c)| (u32::from(c), index)).collect())
}

fn collator(value: Option<&Value>) -> Result<CollatorBorrowed<'static>, String> {
    let tag = match value {
        None => "en-US",
        Some(Value::String(tag)) => tag.as_str(),
        Some(Value::Array(tags)) => match tags.first() {
            None => "en-US",
            Some(Value::String(tag)) => tag.as_str(),
            Some(_) => return Err("\"locales\" must be a string or an array of strings".into()),
        },
        Some(_) => return Err("\"locales\" must be a string or an array of strings".into()),
    };
    let locale = Locale::try_from_str(tag).map_err(|e| format!("invalid locale \"{tag}\": {e}"))?;

    CollatorBorrowed::try_new((&locale).into(), CollatorOptions::default()).map_err(|e| format!("no collation data for \"{tag}\": {e}"))
}

fn comment_match(value: &Value, key: &str) -> Result<CommentMatch, String> {
    match value {
        Value::Bool(value) => Ok(CommentMatch::Bool(*value)),
        other => Ok(CommentMatch::Regex(RegexOption::parse(other, false, key)?)),
    }
}

fn partition_comment(value: Option<&Value>) -> Result<PartitionComment, String> {
    const KEY: &str = "partitionByComment";

    match value {
        None | Some(Value::Bool(false)) => Ok(PartitionComment::Off),
        Some(Value::String(text)) if text.is_empty() => Ok(PartitionComment::Off),
        Some(Value::Object(map)) if !map.contains_key("pattern") => {
            if map.is_empty() {
                return Err(format!("\"{KEY}\" objects need \"block\" or \"line\""));
            }

            let mut block = None;
            let mut line = None;

            for (name, value) in map {
                match name.as_str() {
                    "block" => block = Some(comment_match(value, "partitionByComment.block")?),
                    "line" => line = Some(comment_match(value, "partitionByComment.line")?),
                    other => return Err(format!("\"{KEY}\": unexpected \"{other}\"")),
                }
            }

            Ok(PartitionComment::Split { block, line })
        }
        Some(value) => Ok(PartitionComment::Any(comment_match(value, KEY)?)),
    }
}

fn names(value: &Value, key: &str) -> Result<Names, String> {
    match value {
        Value::String(name) => Ok(Names::One(name.clone())),
        Value::Array(items) if !items.is_empty() => Ok(Names::Many(items.iter().map(|item| string(item, key).map(str::to_owned)).collect::<Result<_, _>>()?)),
        _ => Err(format!("\"{key}\" must be a group name or a non-empty array of them")),
    }
}

fn groups(value: Option<&Value>, sort_by: bool) -> Result<Vec<Group>, String> {
    let Some(value) = value else { return Ok(Vec::new()) };
    let mut groups = Vec::new();

    for item in array(value, "groups")? {
        let group = match item {
            Value::String(_) | Value::Array(_) => Group::Names(names(item, "groups")?),
            Value::Object(map) if map.contains_key("newlinesBetween") && !map.contains_key("group") => {
                if map.len() != 1 {
                    return Err("\"groups\": `newlinesBetween` objects take nothing else".into());
                }

                Group::Newlines(newlines(&map["newlinesBetween"], "groups.newlinesBetween")?)
            }
            Value::Object(map) => {
                let names = names(map.get("group").ok_or("\"groups\" objects need a \"group\"")?, "groups.group")?;

                if map.len() < 2 {
                    return Err("\"groups\" objects need an option besides \"group\"".into());
                }

                let mut sort = SortPatch::default();
                let mut fallback_patch = SortPatch::default();
                let mut newlines_inside = None;

                for (name, value) in map {
                    match name.as_str() {
                        "group" | "commentAbove" => {}
                        "type" => sort.kind = Some(kind(value, "groups.type")?),
                        "order" => sort.order = Some(order(value, "groups.order")?),
                        "sortBy" if sort_by => sort.by = Some(by(value, "groups.sortBy")?),
                        "fallbackSort" => fallback_patch = fallback(value, sort_by, "groups.fallbackSort")?,
                        "newlinesInside" => newlines_inside = Some(newlines(value, "groups.newlinesInside")?),
                        other => return Err(format!("\"groups\": unexpected \"{other}\"")),
                    }
                }

                if let Some(comment) = map.get("commentAbove") {
                    string(comment, "groups.commentAbove")?;
                }

                Group::Override { names, sort, fallback: fallback_patch, newlines_inside }
            }
            _ => return Err("\"groups\" items must be strings, arrays of strings or objects".into()),
        };

        groups.push(group);
    }

    Ok(groups)
}

fn matcher(schema: &Schema, map: &Map<String, Value>, skip: &[&str]) -> Result<Matcher, String> {
    let mut matcher = Matcher::default();

    for (name, value) in map {
        match name.as_str() {
            key if skip.contains(&key) => {}
            "elementNamePattern" => matcher.name = Some(RegexOption::parse(value, false, "elementNamePattern")?),
            "elementValuePattern" if schema.match_keys.contains(&"elementValuePattern") => {
                matcher.value = Some(RegexOption::parse(value, false, "elementValuePattern")?);
            }
            "selector" if schema.match_keys.contains(&"selector") => {
                let selector = string(value, "selector")?;

                if !schema.selectors.contains(&selector) {
                    return Err(format!("\"selector\" must be one of {}", schema.selectors.join(", ")));
                }

                matcher.selector = Some(selector.to_owned());
            }
            "modifiers" if schema.match_keys.contains(&"modifiers") => {
                let mut modifiers = Vec::new();

                for item in array(value, "modifiers")? {
                    let modifier = string(item, "modifiers")?;

                    if !schema.modifiers.contains(&modifier) {
                        return Err(format!("\"modifiers\" items must be one of {}", schema.modifiers.join(", ")));
                    }

                    modifiers.push(modifier.to_owned());
                }

                matcher.modifiers = Some(modifiers);
            }
            other => return Err(format!("\"customGroups\": unexpected \"{other}\"")),
        }
    }

    Ok(matcher)
}

fn custom_groups(schema: &Schema, value: Option<&Value>) -> Result<Vec<CustomGroup>, String> {
    const COMMON: [&str; 6] = ["groupName", "type", "order", "sortBy", "fallbackSort", "newlinesInside"];

    let Some(value) = value else { return Ok(Vec::new()) };
    let mut groups = Vec::new();

    for item in array(value, "customGroups")? {
        let map = object(item, "customGroups")?;
        let name = string(map.get("groupName").ok_or("\"customGroups\" items need a \"groupName\"")?, "groupName")?.to_owned();
        let mut sort = SortPatch::default();
        let mut fallback_patch = SortPatch::default();
        let mut newlines_inside = None;

        for (key, value) in map {
            match key.as_str() {
                "type" => sort.kind = Some(kind(value, "customGroups.type")?),
                "order" => sort.order = Some(order(value, "customGroups.order")?),
                "sortBy" if schema.sort_by => sort.by = Some(by(value, "customGroups.sortBy")?),
                "sortBy" => return Err("\"customGroups\": unexpected \"sortBy\"".into()),
                "fallbackSort" => fallback_patch = fallback(value, schema.sort_by, "customGroups.fallbackSort")?,
                "newlinesInside" => newlines_inside = Some(newlines(value, "customGroups.newlinesInside")?),
                _ => {}
            }
        }

        let matchers = if let Some(any_of) = map.get("anyOf") {
            let items = array(any_of, "anyOf")?;

            if items.is_empty() {
                return Err("\"anyOf\" must not be empty".into());
            }

            let mut skip = COMMON.to_vec();

            skip.push("anyOf");

            if let Some(other) = map.keys().find(|key| !skip.contains(&key.as_str())) {
                return Err(format!("\"customGroups\": unexpected \"{other}\" next to \"anyOf\""));
            }

            items.iter().map(|item| matcher(schema, object(item, "anyOf")?, &[])).collect::<Result<_, _>>()?
        } else {
            if map.len() < 2 {
                return Err("\"customGroups\" items need an option besides \"groupName\"".into());
            }

            vec![matcher(schema, map, &COMMON)?]
        };

        groups.push(CustomGroup { name, matchers, sort, fallback: fallback_patch, newlines_inside });
    }

    Ok(groups)
}

fn when(schema: &Schema, value: Option<&Value>) -> Result<When, String> {
    let Some(value) = value else { return Ok(When::default()) };
    let map = object(value, "useConfigurationIf")?;
    let mut when = When::default();

    for (key, value) in map {
        if key != "allNamesMatchPattern" && !schema.when_keys.contains(&key.as_str()) {
            return Err(format!("\"useConfigurationIf\": unexpected \"{key}\""));
        }

        match key.as_str() {
            "allNamesMatchPattern" => when.all_names = Some(RegexOption::parse(value, false, key)?),
            "matchesAstSelector" => when.selector = Some(Selector::parse(string(value, key)?)?),
            "tagMatchesPattern" => when.tag = Some(RegexOption::parse(value, false, key)?),
            "hasNumericKeysOnly" => when.numeric_keys_only = boolean(Some(value), key)?,
            "declarationCommentMatchesPattern" => when.declaration_comment = Some(RegexOption::parse(value, true, key)?),
            "declarationMatchesPattern" => when.declaration = Some(RegexOption::parse(value, true, key)?),
            "callingFunctionNamePattern" => when.calling_function = Some(RegexOption::parse(value, true, key)?),
            "objectType" => {
                when.object_type = Some(match value.as_str() {
                    Some("destructured") => ObjectType::Destructured,
                    Some("non-destructured") => ObjectType::NonDestructured,
                    _ => return Err("\"objectType\" must be \"destructured\" or \"non-destructured\"".into()),
                });
            }
            _ => {}
        }
    }

    Ok(when)
}

fn validate(schema: &Schema, options: &Options) -> Result<(), String> {
    let uses_custom = options.sort.kind == Kind::Custom
        || options.groups.iter().any(|group| matches!(group, Group::Override { sort, .. } if sort.kind == Some(Kind::Custom)));

    if uses_custom && options.text.alphabet.is_empty() {
        return Err("`alphabet` option must not be empty".into());
    }

    let custom_names: FxHashSet<&str> = options.custom_groups.iter().map(|custom| custom.name.as_str()).collect();
    let all_names: Vec<&str> = options.groups.iter().filter_map(Group::names).flat_map(Names::all).collect();
    let invalid: Vec<&str> = all_names.iter().copied().filter(|name| !is_predefined(schema, name) && !custom_names.contains(name)).collect();

    if !invalid.is_empty() {
        return Err(format!("Invalid group(s): {}", invalid.join(", ")));
    }

    let mut seen = FxHashSet::default();
    let mut duplicated: Vec<&str> = Vec::new();

    for name in &all_names {
        if !seen.insert(*name) && !duplicated.contains(name) {
            duplicated.push(name);
        }
    }

    if !duplicated.is_empty() {
        return Err(format!("Duplicated group(s): {}", duplicated.join(", ")));
    }

    let mut previous_newlines = false;

    for group in &options.groups {
        let is_newlines = group.names().is_none();

        if is_newlines && previous_newlines {
            return Err("Consecutive `newlinesBetween` objects are not allowed".into());
        }

        previous_newlines = is_newlines;
    }

    if options.partition_newline {
        let between_error = "The 'partitionByNewLine' and 'newlinesBetween' options cannot be used together";
        let inside_error = "The 'partitionByNewLine' and 'newlinesInside' options cannot be used together";

        if options.newlines_between != Newlines::Ignore || options.groups.iter().any(|group| matches!(group, Group::Newlines(n) if *n != Newlines::Ignore)) {
            return Err(between_error.into());
        }

        let not_ignored = |value: Option<Newlines>| value.is_some_and(|value| value != Newlines::Ignore);

        if matches!(options.newlines_inside, Inside::Count(_))
            || options.custom_groups.iter().any(|custom| not_ignored(custom.newlines_inside))
            || options.groups.iter().any(|group| matches!(group, Group::Override { newlines_inside, .. } if not_ignored(*newlines_inside)))
        {
            return Err(inside_error.into());
        }
    }

    Ok(())
}

/// `isPredefinedGroup`.
fn is_predefined(schema: &Schema, input: &str) -> bool {
    if input == "unknown" {
        return true;
    }

    let words: Vec<&str> = input.split('-').collect();
    let Some(selector) = longest_word(&words, schema.selectors) else { return false };
    let mut rest = &words[..words.len() - selector.1];
    let mut parsed = FxHashSet::default();

    while !rest.is_empty() {
        let Some((word, count)) = longest_word(rest, schema.modifiers) else { return false };

        if !parsed.insert(word) {
            return false;
        }

        rest = &rest[..rest.len() - count];
    }

    true
}

fn longest_word(words: &[&str], allowed: &[&str]) -> Option<(String, usize)> {
    (1..=3)
        .rev()
        .filter(|&count| words.len() >= count)
        .map(|count| (words[words.len() - count..].join("-"), count))
        .find(|(word, _)| !word.is_empty() && allowed.contains(&word.as_str()))
}
