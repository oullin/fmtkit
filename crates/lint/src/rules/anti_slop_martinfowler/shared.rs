//! What the `anti-slop-martinfowler/*` rules share, ported from upstream's
//! `martinfowler/shared.ts`: option reading, test-file skipping, the ESTree
//! view of wrappers and member expressions, function names and line spans.
//!
//! Upstream reads oxlint's ESTree, which has no parentheses and spells
//! `ChainExpression`, TypeScript assertions and non-null assertions as
//! wrappers. Every helper here takes and returns [`AstKind`]s, so the rules
//! can unwrap through a `ChainExpression` the way upstream does.

use oxc_ast::AstKind;
use oxc_ast::ast::{Argument, BindingPattern, ChainElement, Expression, FunctionType, PropertyKey, SimpleAssignmentTarget, VariableDeclarator};
use oxc_semantic::{AstNode, AstNodes, NodeId, Semantic};
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use serde_json::{Map, Value};

use super::super::anti_slop::shared::{is_js_space, parent, strip_parens};

/// The options object a rule was given, checked against upstream's schema:
/// at most one object, with no keys besides `keys`.
pub struct Options<'v>(Option<&'v Map<String, Value>>);

impl<'v> Options<'v> {
    pub fn parse(options: &'v [Value], keys: &[&str]) -> Result<Self, String> {
        match options {
            [] => Ok(Self(None)),
            [Value::Object(object)] => {
                if let Some(unknown) = object.keys().find(|key| !keys.contains(&key.as_str())) {
                    return Err(format!("unknown option \"{unknown}\""));
                }

                Ok(Self(Some(object)))
            }
            [_] => Err("expected an options object".into()),
            _ => Err("expected at most one options object".into()),
        }
    }

    fn get(&self, key: &str) -> Option<&'v Value> {
        self.0.and_then(|object| object.get(key))
    }

    pub fn boolean(&self, key: &str, default: bool) -> Result<bool, String> {
        self.get(key).map_or(Ok(default), |value| value.as_bool().ok_or_else(|| format!("\"{key}\" must be a boolean")))
    }

    /// A JSON-schema `integer` of at least `minimum`.
    pub fn integer(&self, key: &str, minimum: i64, default: i64) -> Result<i64, String> {
        let Some(value) = self.get(key) else {
            return Ok(default);
        };

        let integer = value.as_i64().or_else(|| {
            value.as_f64().filter(|float| float.fract() == 0.0 && float.abs() < 9_007_199_254_740_992.0).map(
                #[expect(clippy::cast_possible_truncation, reason = "the value is integral and below 2^53")]
                |float| float as i64,
            )
        });

        match integer {
            Some(integer) if integer >= minimum => Ok(integer),
            _ => Err(format!("\"{key}\" must be an integer of at least {minimum}")),
        }
    }

    pub fn strings(&self, key: &str, default: &[&str]) -> Result<Vec<String>, String> {
        let Some(value) = self.get(key) else {
            return Ok(default.iter().map(|item| (*item).to_owned()).collect());
        };

        let message = || format!("\"{key}\" must be an array of strings");
        let items = value.as_array().ok_or_else(message)?;

        items.iter().map(|item| item.as_str().map(str::to_owned).ok_or_else(message)).collect()
    }
}

/// Whether `rel` is a test file by upstream's pattern
/// `(?:^|[\\/])(?:__tests__|__mocks__|test|tests)[\\/]|\.(?:test|spec)\.[cm]?[jt]sx?$`:
/// a test directory anywhere in the path, or a `.test`/`.spec` script.
pub fn is_test_file(rel: &str) -> bool {
    let mut segments: Vec<&str> = rel.split(['/', '\\']).collect();
    let file = segments.pop().unwrap_or_default();

    if segments.iter().any(|segment| matches!(*segment, "__tests__" | "__mocks__" | "test" | "tests")) {
        return true;
    }

    let Some((stem, extension)) = file.rsplit_once('.') else {
        return false;
    };

    let extension = extension.strip_prefix(['c', 'm']).unwrap_or(extension);
    let extension = extension.strip_suffix('x').unwrap_or(extension);

    matches!(extension, "js" | "ts") && stem.rsplit_once('.').is_some_and(|(_, kind)| matches!(kind, "test" | "spec"))
}

/// The ESTree node for an expression.
pub fn expression<'a>(expression: &'a Expression<'a>) -> AstKind<'a> {
    AstKind::from_expression(expression)
}

/// The ESTree node for the expression a `ChainExpression` wraps.
pub fn chain_element<'a>(element: &'a ChainElement<'a>) -> AstKind<'a> {
    match element {
        ChainElement::CallExpression(call) => AstKind::CallExpression(call),
        ChainElement::TSNonNullExpression(non_null) => AstKind::TSNonNullExpression(non_null),
        ChainElement::ComputedMemberExpression(member) => AstKind::ComputedMemberExpression(member),
        ChainElement::StaticMemberExpression(member) => AstKind::StaticMemberExpression(member),
        ChainElement::PrivateFieldExpression(member) => AstKind::PrivateFieldExpression(member),
    }
}

/// The expression a wrapper holds: parentheses, TypeScript assertions and
/// `ChainExpression`, as upstream's `isWrapper`.
fn wrapped(kind: AstKind<'_>) -> Option<AstKind<'_>> {
    match kind {
        AstKind::ParenthesizedExpression(inner) => Some(expression(&inner.expression)),
        AstKind::TSAsExpression(inner) => Some(expression(&inner.expression)),
        AstKind::TSSatisfiesExpression(inner) => Some(expression(&inner.expression)),
        AstKind::TSTypeAssertion(inner) => Some(expression(&inner.expression)),
        AstKind::TSNonNullExpression(inner) => Some(expression(&inner.expression)),
        AstKind::ChainExpression(inner) => Some(chain_element(&inner.expression)),
        _ => None,
    }
}

/// The ESTree node for a simple assignment target.
pub fn simple_target<'a>(target: &'a SimpleAssignmentTarget<'a>) -> AstKind<'a> {
    match target {
        SimpleAssignmentTarget::AssignmentTargetIdentifier(identifier) => AstKind::IdentifierReference(identifier),
        SimpleAssignmentTarget::TSAsExpression(inner) => AstKind::TSAsExpression(inner),
        SimpleAssignmentTarget::TSSatisfiesExpression(inner) => AstKind::TSSatisfiesExpression(inner),
        SimpleAssignmentTarget::TSNonNullExpression(inner) => AstKind::TSNonNullExpression(inner),
        SimpleAssignmentTarget::TSTypeAssertion(inner) => AstKind::TSTypeAssertion(inner),
        SimpleAssignmentTarget::ComputedMemberExpression(member) => AstKind::ComputedMemberExpression(member),
        SimpleAssignmentTarget::StaticMemberExpression(member) => AstKind::StaticMemberExpression(member),
        SimpleAssignmentTarget::PrivateFieldExpression(member) => AstKind::PrivateFieldExpression(member),
    }
}

pub fn is_wrapper(kind: AstKind<'_>) -> bool {
    wrapped(kind).is_some()
}

/// `kind` without parentheses, TypeScript-only wrappers and optional-chain
/// wrappers, as upstream's `unwrap`.
pub fn unwrap(mut kind: AstKind<'_>) -> AstKind<'_> {
    while let Some(inner) = wrapped(kind) {
        kind = inner;
    }

    kind
}

/// The ESTree node of `node`'s nearest ancestor that is not a wrapper.
pub fn parent_past_wrappers<'s, 'a>(nodes: &'s AstNodes<'a>, node: NodeId) -> &'s AstNode<'a> {
    let mut parent_node = parent(nodes, node);

    while is_wrapper(parent_node.kind()) {
        parent_node = parent(nodes, parent_node.id());
    }

    parent_node
}

/// Whether `expression`, as ESTree has it (no parentheses), is the node `id`.
pub fn is_node(expression: &Expression<'_>, id: NodeId) -> bool {
    strip_parens(expression).node_id() == id
}

/// One ESTree `MemberExpression`.
#[derive(Clone, Copy)]
pub struct Member<'a> {
    pub object: &'a Expression<'a>,
    pub property: Property<'a>,
}

#[derive(Clone, Copy)]
pub enum Property<'a> {
    /// `a.name`
    Name(&'a str),
    /// `a.#name`
    Private(&'a str),
    /// `a[expression]`
    Computed(&'a Expression<'a>),
}

impl<'a> Member<'a> {
    pub fn of(kind: AstKind<'a>) -> Option<Self> {
        match kind {
            AstKind::StaticMemberExpression(member) => Some(Self { object: &member.object, property: Property::Name(member.property.name.as_str()) }),
            AstKind::PrivateFieldExpression(member) => Some(Self { object: &member.object, property: Property::Private(member.field.name.as_str()) }),
            AstKind::ComputedMemberExpression(member) => Some(Self { object: &member.object, property: Property::Computed(&member.expression) }),
            _ => None,
        }
    }

    pub fn is_computed(self) -> bool {
        matches!(self.property, Property::Computed(_))
    }

    /// The property name of a non-computed member whose property is an
    /// `Identifier` (not a `PrivateIdentifier`).
    pub fn identifier_name(self) -> Option<&'a str> {
        match self.property {
            Property::Name(name) => Some(name),
            _ => None,
        }
    }
}

/// Whether `kind` is an ESTree `Literal`.
pub fn is_literal(kind: AstKind<'_>) -> bool {
    matches!(
        kind,
        AstKind::BooleanLiteral(_)
            | AstKind::NullLiteral(_)
            | AstKind::NumericLiteral(_)
            | AstKind::BigIntLiteral(_)
            | AstKind::RegExpLiteral(_)
            | AstKind::StringLiteral(_)
    )
}

/// The function a node is, as upstream's `isFunctionNode`: a function
/// declaration, function expression or arrow, not a bodyless TypeScript one.
pub fn is_function_node(kind: AstKind<'_>) -> bool {
    match kind {
        AstKind::Function(function) => {
            matches!(function.r#type, FunctionType::FunctionDeclaration | FunctionType::FunctionExpression) && function.body.is_some()
        }
        AstKind::ArrowFunctionExpression(_) => true,
        _ => false,
    }
}

/// The span ESTree gives a declarator's `id`: it reaches the end of its type
/// annotation.
pub fn declarator_id_span(declarator: &VariableDeclarator<'_>) -> Span {
    let span = declarator.id.span();

    declarator.type_annotation.as_ref().map_or(span, |annotation| Span::new(span.start, annotation.span.end))
}

/// The span ESTree gives a variable's first declaring identifier
/// (`variable.identifiers[0]`).
pub fn binding_span(semantic: &Semantic<'_>, symbol: SymbolId) -> Span {
    let scoping = semantic.scoping();
    let span = scoping.symbol_span(symbol);

    match semantic.nodes().kind(scoping.symbol_declaration(symbol)) {
        AstKind::VariableDeclarator(declarator) if matches!(&declarator.id, BindingPattern::BindingIdentifier(id) if id.span == span) => {
            declarator_id_span(declarator)
        }
        _ => span,
    }
}

/// How many lines `span` covers, counting line breaks as ESLint's
/// `/\r\n|[\r\n  ]/`.
pub fn line_span(source: &str, span: Span) -> usize {
    let text = source.get(span.start as usize..span.end as usize).unwrap_or_default();
    let mut lines = 1;
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                chars.next_if_eq(&'\n');
                lines += 1;
            }
            '\n' | '\u{2028}' | '\u{2029}' => lines += 1,
            _ => {}
        }
    }

    lines
}

/// The source text of `span`.
pub fn text(source: &str, span: Span) -> &str {
    source.get(span.start as usize..span.end as usize).unwrap_or_default()
}

/// `text` with each run of whitespace replaced by one space, as
/// `text.replace(/\s+/g, " ")`.
pub fn collapse_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_space = false;

    for c in text.chars() {
        if is_js_space(c) {
            if !in_space {
                out.push(' ');
            }

            in_space = true;
        } else {
            out.push(c);
            in_space = false;
        }
    }

    out
}

/// Upstream's `truncate`: whitespace collapsed, then cut to `max` UTF-16
/// code units with an ellipsis. A surrogate pair cut in half leaves a lone
/// surrogate in JavaScript, which reaches the diagnostic as U+FFFD.
pub fn truncate(text: &str, max: usize) -> String {
    let compact = collapse_whitespace(text);

    if compact.encode_utf16().count() <= max {
        return compact;
    }

    let mut out = String::new();
    let mut units = 0;
    let limit = max.saturating_sub(1);

    for c in compact.chars() {
        let width = c.len_utf16();

        if units + width > limit {
            if units < limit {
                out.push('\u{fffd}');
            }

            break;
        }

        out.push(c);
        units += width;
    }

    out.push('…');
    out
}

fn static_key_name<'a>(key: &'a PropertyKey<'a>, computed: bool) -> Option<String> {
    if computed {
        return None;
    }

    match key {
        PropertyKey::StaticIdentifier(identifier) => Some(identifier.name.to_string()),
        PropertyKey::PrivateIdentifier(identifier) => Some(format!("#{}", identifier.name)),
        PropertyKey::StringLiteral(literal) => Some(literal.value.to_string()),
        _ => None,
    }
}

/// The string a call's first argument holds: a string literal, or a template
/// with no substitutions.
fn string_argument(arguments: &[Argument<'_>]) -> Option<String> {
    match arguments.first()?.as_expression().map(strip_parens)? {
        Expression::StringLiteral(literal) => Some(literal.value.to_string()),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => {
            template.quasis.first().and_then(|quasi| quasi.value.cooked.as_ref()).map(ToString::to_string)
        }
        _ => None,
    }
}

/// Upstream's `functionName`: a function's own id, or the binding, key,
/// assignment target or `Effect.fn("name")` that holds it.
pub fn function_name(semantic: &Semantic<'_>, function: &AstNode<'_>) -> Option<String> {
    if let AstKind::Function(function) = function.kind()
        && let Some(id) = &function.id
    {
        return Some(id.name.to_string());
    }

    let nodes = semantic.nodes();
    let mut holder = function.id();
    let mut parent_node = parent(nodes, holder);

    while is_wrapper(parent_node.kind()) {
        holder = parent_node.id();
        parent_node = parent(nodes, holder);
    }

    match parent_node.kind() {
        AstKind::VariableDeclarator(declarator) => match &declarator.id {
            BindingPattern::BindingIdentifier(id) => Some(id.name.to_string()),
            _ => None,
        },
        AstKind::MethodDefinition(method) => static_key_name(&method.key, method.computed),
        AstKind::ObjectProperty(property) => static_key_name(&property.key, property.computed),
        AstKind::PropertyDefinition(property) => static_key_name(&property.key, property.computed),
        AstKind::AssignmentExpression(assignment) if is_node(&assignment.right, holder) => {
            Some(text(semantic.source_text(), assignment.left.span()).to_owned())
        }
        AstKind::CallExpression(call) if !is_node(&call.callee, holder) => {
            let callee = unwrap(expression(&call.callee));

            if let AstKind::CallExpression(inner) = callee
                && let Some(label) = string_argument(&inner.arguments)
            {
                return Some(label);
            }

            let traced = match callee {
                AstKind::CallExpression(inner) => unwrap(expression(&inner.callee)),
                _ => callee,
            };

            let is_effect_constructor = Member::of(traced).and_then(Member::identifier_name).is_some_and(|name| matches!(name, "fn" | "fnUntraced" | "gen"));

            if !is_effect_constructor {
                return None;
            }

            let mut declarator = parent(nodes, parent_node.id());

            while matches!(declarator.kind(), AstKind::CallExpression(_)) || is_wrapper(declarator.kind()) {
                declarator = parent(nodes, declarator.id());
            }

            match declarator.kind() {
                AstKind::VariableDeclarator(declarator) => match &declarator.id {
                    BindingPattern::BindingIdentifier(id) => Some(id.name.to_string()),
                    _ => None,
                },
                _ => None,
            }
        }
        _ => None,
    }
}

/// Upstream's `functionReportNode`: a function's name when it has one,
/// otherwise the function.
pub fn function_report_span(nodes: &AstNodes<'_>, function: &AstNode<'_>) -> Span {
    if let AstKind::Function(function) = function.kind()
        && let Some(id) = &function.id
    {
        return id.span;
    }

    let key_span = |key: &PropertyKey<'_>| key.as_expression().map_or_else(|| key.span(), |key| strip_parens(key).span());

    match parent_past_wrappers(nodes, function.id()).kind() {
        AstKind::VariableDeclarator(declarator) => declarator_id_span(declarator),
        AstKind::MethodDefinition(method) => key_span(&method.key),
        AstKind::ObjectProperty(property) => key_span(&property.key),
        AstKind::PropertyDefinition(property) => key_span(&property.key),
        _ => function.kind().span(),
    }
}

/// Upstream's `describeFunction`.
pub fn describe_function(name: Option<&str>) -> String {
    name.map_or_else(|| "This function".to_owned(), |name| format!("`{name}`"))
}

#[cfg(test)]
mod tests {
    use super::{is_test_file, truncate};

    #[test]
    fn test_files_match_upstreams_pattern() {
        for path in ["src/a.test.ts", "a.spec.mjsx", "src/__tests__/handler.ts", "test/a.ts", "a\\tests\\b.js", "x/__mocks__/y.ts", ".test.ts"] {
            assert!(is_test_file(path), "{path}");
        }

        for path in ["file.ts", "src/test.ts", "src/tests", "a.test.ts.bak", "a.test.json", "contest/a.ts", "a.tests.ts"] {
            assert!(!is_test_file(path), "{path}");
        }
    }

    #[test]
    fn truncate_counts_utf16_units() {
        assert_eq!(truncate("a  b\n\tc", 80), "a b c");
        assert_eq!(truncate("abcdef", 4), "abc…");
        assert_eq!(truncate("ab😀def", 4), "ab\u{fffd}…");
        assert_eq!(truncate("a😀def", 4), "a😀…");
    }
}
