//! Type binders in scope at a node, which can shadow module-level aliases.

use oxc_ast::AstKind;
use oxc_ast::ast::{TSInferType, TSTypeParameterDeclaration};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::{AstNodes, NodeId};
use oxc_span::{GetSpan, Span};
use rustc_hash::FxHashSet;

use super::{ancestors, strip_type_parens};

/// Every type parameter, mapped-type key and `infer` binder visible at `id`,
/// the node itself included, as v1's `lexicalTypeParameterNames`.
pub fn type_parameter_names<'a>(nodes: &AstNodes<'a>, id: NodeId) -> FxHashSet<&'a str> {
    let mut names = FxHashSet::default();
    let mut descendant: Option<Span> = None;

    for node in std::iter::once(nodes.get_node(id)).chain(ancestors(nodes, id)) {
        let kind = node.kind();

        if let Some(parameters) = type_parameters(kind) {
            names.extend(parameters.params.iter().map(|parameter| parameter.name.name.as_str()));
        }

        match kind {
            AstKind::TSMappedType(mapped) => {
                let is_value = |ty: Option<&oxc_ast::ast::TSType<'_>>| ty.is_some_and(|ty| Some(strip_type_parens(ty).span()) == descendant);

                if is_value(mapped.name_type.as_ref()) || is_value(mapped.type_annotation.as_ref()) {
                    names.insert(mapped.key.name.as_str());
                }
            }
            AstKind::TSConditionalType(conditional) if Some(strip_type_parens(&conditional.true_type).span()) == descendant => {
                InferNames(&mut names).visit_ts_type(&conditional.extends_type);
            }
            _ => {}
        }

        descendant = Some(kind.span());
    }

    names
}

fn type_parameters(kind: AstKind<'_>) -> Option<&TSTypeParameterDeclaration<'_>> {
    match kind {
        AstKind::Function(it) => it.type_parameters.as_deref(),
        AstKind::ArrowFunctionExpression(it) => it.type_parameters.as_deref(),
        AstKind::Class(it) => it.type_parameters.as_deref(),
        AstKind::TSInterfaceDeclaration(it) => it.type_parameters.as_deref(),
        AstKind::TSTypeAliasDeclaration(it) => it.type_parameters.as_deref(),
        AstKind::TSCallSignatureDeclaration(it) => it.type_parameters.as_deref(),
        AstKind::TSConstructSignatureDeclaration(it) => it.type_parameters.as_deref(),
        AstKind::TSMethodSignature(it) => it.type_parameters.as_deref(),
        AstKind::TSFunctionType(it) => it.type_parameters.as_deref(),
        AstKind::TSConstructorType(it) => it.type_parameters.as_deref(),
        _ => None,
    }
}

struct InferNames<'s, 'a>(&'s mut FxHashSet<&'a str>);

impl<'a> Visit<'a> for InferNames<'_, 'a> {
    fn visit_ts_infer_type(&mut self, it: &TSInferType<'a>) {
        self.0.insert(it.type_parameter.name.name.as_str());
        walk::walk_ts_infer_type(self, it);
    }
}
