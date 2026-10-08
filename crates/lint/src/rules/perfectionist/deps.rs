//! `computeDependenciesBySortingNode`: which sorting nodes each sorting node
//! references, through the scope analysis.

use oxc_ast::AstKind;
use oxc_semantic::{NodeId, Semantic};
use oxc_span::Span;

use super::sort::Item;

/// One resolved reference inside a sorting node.
pub struct Reference<'a> {
    /// The identifier's node.
    pub node: NodeId,
    pub name: &'a str,
    /// The sorting node holding the identifier.
    pub referencing: usize,
}

fn containing(spans: &[Span], span: Span) -> Option<usize> {
    spans.iter().position(|outer| outer.start <= span.start && outer.end >= span.end)
}

/// The sorting nodes each sorting node depends on, from the identifiers among
/// `candidates`: the node declaring each resolved identifier unless `ignore`
/// skips it, plus whatever `additional` returns.
pub fn referenced<'a>(
    semantic: &Semantic<'a>,
    candidates: &[NodeId],
    spans: &[Span],
    ignore: impl Fn(&Reference<'a>) -> bool,
    additional: impl Fn(&Reference<'a>) -> Vec<usize>,
) -> Vec<Vec<usize>> {
    let mut found = vec![Vec::new(); spans.len()];
    let scoping = semantic.scoping();

    for &id in candidates {
        let AstKind::IdentifierReference(identifier) = semantic.nodes().kind(id) else { continue };

        let Some(symbol) = identifier.reference_id.get().and_then(|id| scoping.get_reference(id).symbol_id()) else { continue };
        let Some(referencing) = containing(spans, identifier.span) else { continue };
        let reference = Reference { node: id, name: identifier.name.as_str(), referencing };

        if !ignore(&reference)
            && let Some(declared) = containing(spans, scoping.symbol_span(symbol))
            && declared != referencing
        {
            found[referencing].push(declared);
        }

        let extra = additional(&reference);

        found[referencing].extend(extra);
    }

    found
}

/// `populateSortingNodeGroupsWithDependencies`.
pub fn apply(items: &mut [Item], referenced: &[Vec<usize>]) {
    let dependencies: Vec<Vec<String>> = referenced.iter().map(|nodes| nodes.iter().flat_map(|&node| items[node].dependency_names.clone()).collect()).collect();

    for (item, dependencies) in items.iter_mut().zip(dependencies) {
        item.dependencies = dependencies;
    }
}
