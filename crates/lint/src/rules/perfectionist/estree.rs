//! ESTree's view of oxc's AST: node type names, parents, and the esquery
//! subset `useConfigurationIf.matchesAstSelector` accepts.

use std::borrow::Cow;

use oxc_ast::AstKind;
use oxc_ast::ast::{Expression, FunctionType};
use oxc_semantic::{AstNodes, NodeId};
use oxc_span::GetSpan;

/// The ESTree type of the node, or `None` for oxc nodes ESTree has no node
/// for (parentheses, parameter lists).
pub fn type_name(nodes: &AstNodes<'_>, id: NodeId) -> Option<Cow<'static, str>> {
    let kind = nodes.kind(id);
    let name = match kind {
        AstKind::ParenthesizedExpression(_)
        | AstKind::TSParenthesizedType(_)
        | AstKind::FormalParameters(_)
        | AstKind::FormalParameter(_)
        | AstKind::CatchParameter(_)
        | AstKind::Hashbang(_)
        | AstKind::WithClause(_)
        | AstKind::Elision(_) => return None,
        AstKind::IdentifierName(_)
        | AstKind::IdentifierReference(_)
        | AstKind::BindingIdentifier(_)
        | AstKind::LabelIdentifier(_)
        | AstKind::TSIndexSignatureName(_)
        | AstKind::TSThisParameter(_) => "Identifier",
        AstKind::ObjectProperty(_)
        | AstKind::BindingProperty(_)
        | AstKind::AssignmentTargetPropertyIdentifier(_)
        | AstKind::AssignmentTargetPropertyProperty(_) => "Property",
        AstKind::StaticMemberExpression(_) | AstKind::ComputedMemberExpression(_) | AstKind::PrivateFieldExpression(_) => "MemberExpression",
        AstKind::BooleanLiteral(_)
        | AstKind::NullLiteral(_)
        | AstKind::NumericLiteral(_)
        | AstKind::StringLiteral(_)
        | AstKind::BigIntLiteral(_)
        | AstKind::RegExpLiteral(_) => "Literal",
        AstKind::Function(function) => match function.r#type {
            FunctionType::FunctionDeclaration => "FunctionDeclaration",
            FunctionType::FunctionExpression => "FunctionExpression",
            FunctionType::TSDeclareFunction | FunctionType::TSEmptyBodyFunctionExpression => "TSDeclareFunction",
        },
        AstKind::Class(class) => {
            if class.is_declaration() {
                "ClassDeclaration"
            } else {
                "ClassExpression"
            }
        }
        AstKind::FunctionBody(_) => "BlockStatement",
        AstKind::ObjectAssignmentTarget(_) => "ObjectPattern",
        AstKind::ArrayAssignmentTarget(_) => "ArrayPattern",
        AstKind::AssignmentTargetWithDefault(_) => "AssignmentPattern",
        AstKind::BindingRestElement(_) | AstKind::AssignmentTargetRest(_) | AstKind::FormalParameterRest(_) => "RestElement",
        AstKind::ExportDeclaration(_) | AstKind::ExportFromDeclaration(_) => "ExportNamedDeclaration",
        AstKind::TSExternalModuleDeclaration(_) | AstKind::TSNamespaceDeclaration(_) | AstKind::TSGlobalDeclaration(_) => "TSModuleDeclaration",
        AstKind::TSImportTypeQualifiedName(_) => "TSQualifiedName",
        AstKind::Directive(_) => "ExpressionStatement",
        AstKind::ImportMeta(_) | AstKind::NewTarget(_) => "MetaProperty",
        _ => return Some(Cow::Owned(format!("{:?}", kind.ty()))),
    };

    Some(Cow::Borrowed(name))
}

/// The closest ancestor ESTree has a node for.
pub fn parent(nodes: &AstNodes<'_>, id: NodeId) -> Option<NodeId> {
    nodes.ancestor_ids(id).find(|&ancestor| ancestor != id && type_name(nodes, ancestor).is_some())
}

/// Every ESTree ancestor, closest first.
pub fn ancestors<'n>(nodes: &'n AstNodes<'_>, id: NodeId) -> impl Iterator<Item = NodeId> + 'n {
    nodes.ancestor_ids(id).filter(move |&ancestor| ancestor != id && type_name(nodes, ancestor).is_some())
}

pub fn is_type(nodes: &AstNodes<'_>, id: NodeId, name: &str) -> bool {
    type_name(nodes, id).is_some_and(|found| found == name)
}

/// ESTree's `parent.callee === node` for a call or `new` parent.
pub fn is_callee(nodes: &AstNodes<'_>, function: NodeId) -> bool {
    let span = nodes.kind(function).span();
    let Some(parent) = parent(nodes, function) else { return false };
    let callee = match nodes.kind(parent) {
        AstKind::CallExpression(call) => &call.callee,
        AstKind::NewExpression(call) => &call.callee,
        _ => return false,
    };

    unparenthesized(callee).span() == span
}

pub fn unparenthesized<'b, 'a>(expression: &'b Expression<'a>) -> &'b Expression<'a> {
    expression.without_parentheses()
}

/// A parsed `matchesAstSelector`: alternatives of type chains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    source: String,
    alternatives: Vec<Vec<(Combinator, Compound)>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Combinator {
    Subject,
    Child,
    Descendant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Compound {
    Any,
    Type(String),
}

impl Selector {
    /// Parses the subset of esquery fmtkit supports: node types, `*`, the
    /// child (`>`) and descendant combinators, and `,` alternatives.
    pub fn parse(source: &str) -> Result<Self, String> {
        let unsupported = || format!("unsupported `matchesAstSelector` \"{source}\": fmtkit supports node types, `*`, `>`, descendants and `,`");
        let mut alternatives = Vec::new();

        for alternative in source.split(',') {
            let spaced = alternative.replace('>', " > ");
            let mut chain = Vec::new();
            let mut pending = Combinator::Subject;

            for token in spaced.split_whitespace() {
                if token == ">" {
                    if chain.is_empty() || pending == Combinator::Child {
                        return Err(unsupported());
                    }

                    pending = Combinator::Child;
                    continue;
                }

                let compound = if token == "*" {
                    Compound::Any
                } else if token.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && token.starts_with(|c: char| c.is_ascii_alphabetic()) {
                    Compound::Type(token.to_owned())
                } else {
                    return Err(unsupported());
                };
                let combinator = if chain.is_empty() {
                    Combinator::Subject
                } else if pending == Combinator::Child {
                    Combinator::Child
                } else {
                    Combinator::Descendant
                };

                chain.push((combinator, compound));
                pending = Combinator::Descendant;
            }

            if chain.is_empty() || pending == Combinator::Child {
                return Err(unsupported());
            }

            alternatives.push(chain);
        }

        Ok(Self { source: source.to_owned(), alternatives })
    }

    /// Whether `id` is the subject of a match.
    pub fn matches(&self, nodes: &AstNodes<'_>, id: NodeId) -> bool {
        self.alternatives.iter().any(|chain| matches_chain(nodes, id, chain))
    }
}

fn matches_compound(nodes: &AstNodes<'_>, id: NodeId, compound: &Compound) -> bool {
    match compound {
        Compound::Any => type_name(nodes, id).is_some(),
        Compound::Type(name) => is_type(nodes, id, name),
    }
}

fn matches_chain(nodes: &AstNodes<'_>, id: NodeId, chain: &[(Combinator, Compound)]) -> bool {
    let Some(((combinator, compound), rest)) = chain.split_last() else { return true };

    if !matches_compound(nodes, id, compound) {
        return false;
    }

    match combinator {
        Combinator::Subject => true,
        Combinator::Child => parent(nodes, id).is_some_and(|parent| matches_chain(nodes, parent, rest)),
        Combinator::Descendant => ancestors(nodes, id).any(|ancestor| matches_chain(nodes, ancestor, rest)),
    }
}
