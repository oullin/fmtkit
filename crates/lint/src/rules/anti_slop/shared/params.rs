//! Function-like nodes and their parameters, in ESTree's terms.

use oxc_ast::AstKind;
use oxc_ast::ast::{BindingPattern, FormalParameter, FormalParameterRest, FormalParameters, TSThisParameter, TSTypeAnnotation};
use oxc_span::{GetSpan, Span};

use super::is_js_space;

/// The parameters and return type of a function or a function-like signature.
pub struct Signature<'a> {
    this_param: Option<&'a TSThisParameter<'a>>,
    params: &'a FormalParameters<'a>,
    pub return_type: Option<&'a TSTypeAnnotation<'a>>,
}

impl<'a> Signature<'a> {
    /// The signature of every node v1 visits as a parameter owner: functions of
    /// every kind, arrows, call, construct and method signatures, and function
    /// and constructor types.
    pub fn of(kind: AstKind<'a>) -> Option<Self> {
        let (this_param, params, return_type) = match kind {
            AstKind::Function(it) => (it.this_param.as_deref(), &*it.params, it.return_type.as_deref()),
            AstKind::ArrowFunctionExpression(it) => (None, &*it.params, it.return_type.as_deref()),
            AstKind::TSCallSignatureDeclaration(it) => (it.this_param.as_deref(), &*it.params, it.return_type.as_deref()),
            AstKind::TSConstructSignatureDeclaration(it) => (None, &*it.params, it.return_type.as_deref()),
            AstKind::TSMethodSignature(it) => (it.this_param.as_deref(), &*it.params, it.return_type.as_deref()),
            AstKind::TSFunctionType(it) => (it.this_param.as_deref(), &*it.params, Some(&*it.return_type)),
            AstKind::TSConstructorType(it) => (None, &*it.params, Some(&*it.return_type)),
            _ => return None,
        };

        Some(Self { this_param, params, return_type })
    }

    /// The parameters in ESTree order: `this`, the list, then the rest element.
    pub fn params(&self) -> impl Iterator<Item = Param<'a>> + '_ {
        self.this_param.map(Param::This).into_iter().chain(self.params.items.iter().map(Param::Formal)).chain(self.params.rest.as_deref().map(Param::Rest))
    }
}

/// One parameter.
#[derive(Clone, Copy)]
pub enum Param<'a> {
    This(&'a TSThisParameter<'a>),
    Formal(&'a FormalParameter<'a>),
    Rest(&'a FormalParameterRest<'a>),
}

impl<'a> Param<'a> {
    pub fn annotation(self) -> Option<&'a TSTypeAnnotation<'a>> {
        match self {
            Self::This(it) => it.type_annotation.as_deref(),
            Self::Formal(it) => it.type_annotation.as_deref(),
            Self::Rest(it) => it.type_annotation.as_deref(),
        }
    }

    /// Whether ESTree shows this parameter as a bare `Identifier`.
    fn bare_name(self) -> Option<&'a str> {
        match self {
            Self::This(_) => Some("this"),
            Self::Formal(it) if it.initializer.is_none() && !is_parameter_property(it) => binding_name(&it.pattern),
            _ => None,
        }
    }

    /// The source range of the ESTree parameter node.
    pub fn estree_span(self) -> Span {
        match self {
            Self::This(it) => it.span,
            Self::Formal(it) if it.initializer.is_some() || is_parameter_property(it) => it.span,
            Self::Formal(it) => {
                let end = it.type_annotation.as_ref().map_or(if it.optional { it.span.end } else { it.pattern.span().end }, |annotation| annotation.span.end);

                Span::new(it.pattern.span().start, end)
            }
            Self::Rest(it) => {
                let start = it.decorators.first().map_or(it.rest.span.start, |decorator| decorator.span.start);
                let end = it.type_annotation.as_ref().map_or(it.rest.span.end, |annotation| annotation.span.end);

                Span::new(start, end)
            }
        }
    }

    /// v1's display name in `no-object-parameters`: the identifier, or the
    /// parameter text without a trailing `: keyword`.
    pub fn display_name(self, source: &'a str, keyword: &str) -> &'a str {
        self.bare_name().unwrap_or_else(|| without_annotation(self.estree_span().source_text(source), keyword))
    }

    /// Upstream's `functionParameterBindingName`: the local binding, through a
    /// default, a rest element or a parameter property; a destructuring
    /// pattern is its text up to its own annotation.
    pub fn binding_display_name(self, source: &'a str) -> &'a str {
        let (pattern, annotation) = match self {
            Self::This(_) => return "this",
            Self::Formal(it) => (&it.pattern, it.type_annotation.as_deref()),
            // ESTree hangs a rest parameter's annotation on the rest element, not its argument.
            Self::Rest(it) => (&it.rest.argument, None),
        };

        if let Some(name) = binding_name(pattern) {
            return name;
        }

        let span = pattern.span();
        let end = annotation.map_or(span.end, |annotation| annotation.span.start);

        source.get(span.start as usize..end as usize).map_or("", |text| text.trim_end_matches(is_js_space))
    }
}

fn is_parameter_property(param: &FormalParameter<'_>) -> bool {
    param.accessibility.is_some() || param.readonly || param.r#override
}

fn binding_name<'a>(pattern: &BindingPattern<'a>) -> Option<&'a str> {
    match pattern {
        BindingPattern::BindingIdentifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

/// `text` without a trailing `\s*:\s*keyword\s*`, as v1's regular expression.
fn without_annotation<'t>(text: &'t str, keyword: &str) -> &'t str {
    text.trim_end_matches(is_js_space)
        .strip_suffix(keyword)
        .and_then(|rest| rest.trim_end_matches(is_js_space).strip_suffix(':'))
        .map_or(text, |rest| rest.trim_end_matches(is_js_space))
}
