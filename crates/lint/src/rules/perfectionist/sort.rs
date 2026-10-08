//! Sorting nodes: perfectionist's comparators, `sortNodes`,
//! `sortNodesByGroups` and `sortNodesByDependencies`.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::sync::LazyLock;

use oxc_span::Span;
use regress::Regex;
use rustc_hash::FxHashSet;

use super::jsnum;
use super::natural;
use super::options::{By, Kind, Options, Order, Sort, SortByValue, Spec, Special, Text};
use super::source::is_js_space;

/// One element to sort: perfectionist's `SortingNode`.
#[derive(Debug, Clone, Default)]
pub struct Item {
    pub span: Span,
    pub name: String,
    /// What `sortBy: 'value'` (or enums' `sortByValue`) compares.
    pub value: String,
    /// Enums: the member's computed number.
    pub numeric: Option<f64>,
    pub size: u32,
    pub group: String,
    pub partition: usize,
    pub disabled: bool,
    pub semicolon: bool,
    pub dependency_names: Vec<String>,
    pub dependencies: Vec<String>,
}

/// Which comparator computer a rule uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// `defaultComparatorByOptionsComputer`.
    Name,
    /// `nameOrValueComparatorByOptionsComputer`.
    NameOrValue,
    /// sort-enums' computer.
    Enum { by_value: SortByValue, numeric: bool },
}

static REMOVE: LazyLock<Regex> = LazyLock::new(|| Regex::with_flags(r"[^a-z\u{C0}-\u{24F}\u{1E00}-\u{1EFF}]+", "iu").expect("a valid pattern"));
static TRIM: LazyLock<Regex> = LazyLock::new(|| Regex::with_flags(r"^[^a-z\u{C0}-\u{24F}\u{1E00}-\u{1EFF}]+", "iu").expect("a valid pattern"));

/// `buildStringFormatter`.
pub fn format(text: &Text, value: &str) -> String {
    let lowered;
    let mut value = if text.ignore_case {
        lowered = value.to_lowercase();
        lowered.as_str()
    } else {
        value
    };
    let replaced;

    match text.special {
        Special::Remove => {
            replaced = REMOVE.replace_all(value, "");
            value = &replaced;
        }
        Special::Trim => {
            replaced = TRIM.replace(value, "");
            value = &replaced;
        }
        Special::Keep => {}
    }

    value.chars().filter(|&c| !is_js_space(c)).collect()
}

fn ordered(result: Ordering, order: Order) -> Ordering {
    match order {
        Order::Asc => result,
        Order::Desc => result.reverse(),
    }
}

/// Sorts with one rule's comparators.
pub struct Sorter<'o> {
    pub options: &'o Options,
    pub mode: Mode,
}

impl Sorter<'_> {
    /// One comparator built from `sort`.
    pub fn compare(&self, sort: Sort, a: &Item, b: &Item) -> Ordering {
        match self.mode {
            Mode::Name => self.by_key(sort, By::Name, a, b),
            Mode::NameOrValue => self.by_key(sort, sort.by, a, b),
            Mode::Enum { by_value, numeric } => match by_value {
                SortByValue::IfNumericEnum | SortByValue::Always if numeric => self.by_number(sort, a, b),
                SortByValue::Always => self.by_key(sort, By::Value, a, b),
                SortByValue::IfNumericEnum | SortByValue::Never => self.by_key(sort, By::Name, a, b),
            },
        }
    }

    fn by_number(&self, sort: Sort, a: &Item, b: &Item) -> Ordering {
        match sort.kind {
            Kind::SubgroupOrder => self.by_key(sort, By::Name, a, b),
            Kind::Unsorted => Ordering::Equal,
            _ => {
                let text = &self.options.text;
                let a = format(text, &jsnum::format(a.numeric.unwrap_or(f64::NAN)));
                let b = format(text, &jsnum::format(b.numeric.unwrap_or(f64::NAN)));

                ordered(natural::compare(&a, &b, &text.collator), sort.order)
            }
        }
    }

    fn by_key(&self, sort: Sort, by: By, a: &Item, b: &Item) -> Ordering {
        let key = |item: &'_ Item| -> String {
            match by {
                By::Name => item.name.clone(),
                By::Value => item.value.clone(),
            }
        };
        let text = &self.options.text;
        let result = match sort.kind {
            Kind::Unsorted => return Ordering::Equal,
            Kind::SubgroupOrder => {
                let (Some(x), Some(y)) = (self.options.subgroup(&a.group), self.options.subgroup(&b.group)) else { return Ordering::Equal };

                if x != y {
                    return Ordering::Equal;
                }

                self.options.subgroup_position(x, &a.group).cmp(&self.options.subgroup_position(y, &b.group))
            }
            Kind::LineLength => a.size.cmp(&b.size),
            Kind::Alphabetical => text.collator.compare(&format(text, &key(a)), &format(text, &key(b))),
            Kind::Natural => natural::compare(&format(text, &key(a)), &format(text, &key(b)), &text.collator),
            Kind::Custom => custom(text, &format(text, &key(a)), &format(text, &key(b))),
        };

        ordered(result, sort.order)
    }

    /// `sortNodes`: `ignored` nodes keep their indexes.
    pub fn sort_nodes(&self, items: &[Item], nodes: &[usize], spec: Spec, ignored: impl Fn(usize) -> bool) -> Vec<usize> {
        let mut kept: Vec<usize> = nodes.iter().copied().filter(|&node| !ignored(node)).collect();

        if spec.sort.kind != Kind::Unsorted {
            let fallback = spec.fallback_sort();

            merge_sort(&mut kept, &|&a, &b| {
                let (a, b) = (&items[a], &items[b]);

                self.compare(spec.sort, a, b).then_with(|| self.compare(fallback, a, b))
            });
        }

        for (index, &node) in nodes.iter().enumerate() {
            if ignored(node) {
                kept.insert(index.min(kept.len()), node);
            }
        }

        kept
    }

    /// `sortNodesByGroups`. `ignored_in_group` is `isNodeIgnoredForGroup`.
    pub fn sort_by_groups(&self, items: &[Item], nodes: &[usize], ignore_disabled: bool, ignored_in_group: impl Fn(&Spec, &Item) -> bool) -> Vec<usize> {
        let mut buckets: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        let mut ignored = Vec::new();

        for (index, &node) in nodes.iter().enumerate() {
            if ignore_disabled && items[node].disabled {
                ignored.push(index);
                continue;
            }

            buckets.entry(self.options.group_index(&items[node].group)).or_default().push(node);
        }

        let mut sorted = Vec::with_capacity(nodes.len());

        for (group_index, bucket) in buckets {
            let spec = self.options.spec(group_index);
            let skipped: FxHashSet<usize> = bucket.iter().copied().filter(|&node| ignored_in_group(&spec, &items[node])).collect();

            sorted.extend(self.sort_nodes(items, &bucket, spec, |node| skipped.contains(&node)));
        }

        for index in ignored {
            sorted.insert(index.min(sorted.len()), nodes[index]);
        }

        sorted
    }
}

/// `compareByCustomSort`: by alphabet index per UTF-16 unit, then length.
fn custom(text: &Text, a: &str, b: &str) -> Ordering {
    let index = |unit: u16| text.alphabet.get(&u32::from(unit)).copied();
    let a: Vec<u16> = a.encode_utf16().collect();
    let b: Vec<u16> = b.encode_utf16().collect();

    for (&x, &y) in a.iter().zip(&b) {
        let (x, y) = (index(x), index(y));

        if x != y {
            let greater = match (x, y) {
                (Some(x), Some(y)) => x > y,
                (None, _) => true,
                (Some(_), None) => false,
            };

            return if greater { Ordering::Greater } else { Ordering::Less };
        }
    }

    a.len().cmp(&b.len())
}

/// A stable merge sort that tolerates comparators that are not total orders.
pub fn merge_sort<T: Copy>(items: &mut [T], compare: &dyn Fn(&T, &T) -> Ordering) {
    if items.len() < 2 {
        return;
    }

    let middle = items.len() / 2;

    merge_sort(&mut items[..middle], compare);
    merge_sort(&mut items[middle..], compare);

    let mut merged = Vec::with_capacity(items.len());
    let (mut i, mut j) = (0, middle);

    while i < middle && j < items.len() {
        if compare(&items[j], &items[i]) == Ordering::Less {
            merged.push(items[j]);
            j += 1;
        } else {
            merged.push(items[i]);
            i += 1;
        }
    }

    merged.extend_from_slice(&items[i..middle]);
    merged.extend_from_slice(&items[j..]);
    items.copy_from_slice(&merged);
}

/// `isNodeDependentOnOtherNode`: whether `b` depends on `a`.
pub fn depends(items: &[Item], a: usize, b: usize) -> bool {
    a != b && items[a].dependency_names.iter().any(|name| items[b].dependencies.contains(name))
}

/// `computeNodesInCircularDependencies`.
pub fn circular(items: &[Item], nodes: &[usize]) -> FxHashSet<usize> {
    struct Search<'i> {
        items: &'i [Item],
        nodes: &'i [usize],
        cycles: FxHashSet<usize>,
        visiting: FxHashSet<usize>,
        visited: FxHashSet<usize>,
    }

    impl Search<'_> {
        fn visit(&mut self, element: usize, mut path: Vec<usize>) {
            if self.visited.contains(&element) {
                return;
            }

            if self.visiting.contains(&element) {
                if let Some(start) = path.iter().position(|&node| node == element) {
                    self.cycles.extend(&path[start..]);
                }

                return;
            }

            self.visiting.insert(element);
            path.push(element);

            for dependency in &self.items[element].dependencies {
                let found = self.nodes.iter().copied().find(|&node| node != element && self.items[node].dependency_names.contains(dependency));

                if let Some(found) = found {
                    self.visit(found, path.clone());
                }
            }

            self.visiting.remove(&element);
            self.visited.insert(element);
        }
    }

    let mut search = Search { items, nodes, cycles: FxHashSet::default(), visiting: FxHashSet::default(), visited: FxHashSet::default() };

    for &node in nodes {
        search.visit(node, Vec::new());
    }

    search.cycles
}

/// `sortNodesByDependencies`.
pub fn sort_by_dependencies(items: &[Item], nodes: &[usize], ignore_disabled: bool) -> Vec<usize> {
    fn visit(
        items: &[Item],
        nodes: &[usize],
        cycles: &FxHashSet<usize>,
        ignore_disabled: bool,
        node: usize,
        state: &mut (FxHashSet<usize>, FxHashSet<usize>, Vec<usize>),
    ) {
        if state.0.contains(&node) || !state.1.insert(node) {
            return;
        }

        for &other in nodes {
            if !cycles.contains(&other) && depends(items, other, node) && (!ignore_disabled || !items[other].disabled) {
                visit(items, nodes, cycles, ignore_disabled, other, state);
            }
        }

        state.1.remove(&node);
        state.0.insert(node);
        state.2.push(node);
    }

    let cycles = circular(items, nodes);
    let mut state = (FxHashSet::default(), FxHashSet::default(), Vec::with_capacity(nodes.len()));

    for &node in nodes {
        visit(items, nodes, &cycles, ignore_disabled, node, &mut state);
    }

    state.2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_sort_is_stable() {
        let mut values = [(1, 'a'), (0, 'b'), (1, 'c'), (0, 'd')];

        merge_sort(&mut values, &|a, b| a.0.cmp(&b.0));

        assert_eq!(values, [(0, 'b'), (0, 'd'), (1, 'a'), (1, 'c')]);
    }
}
