//! What the `perf/*` rules share: static regular expressions, the `const` a
//! name is bound to, and the receiver a call chain starts from.

use oxc_ast::AstKind;
use oxc_ast::ast::{Argument, Expression, IdentifierReference, RegExpFlags, VariableDeclarationKind};

use super::super::Context;
use super::super::anti_slop::shared::{is_global, member, strip_parens};

/// Whether there are arguments and every one is a string, or a template with
/// no substitutions: a value known before the call runs.
pub fn static_arguments(arguments: &[Argument<'_>]) -> bool {
    !arguments.is_empty()
        && arguments.iter().all(|argument| match argument {
            Argument::StringLiteral(_) => true,
            Argument::TemplateLiteral(template) => template.expressions.is_empty(),
            _ => false,
        })
}

/// Whether `expression` is a regular expression that keeps `lastIndex`
/// between calls (the `g` or `y` flag): a literal, `new RegExp(…, flags)`, or
/// a `const` in this file bound to either.
pub fn is_stateful_regex<'a>(ctx: &Context<'_, 'a>, expression: &Expression<'a>) -> bool {
    match strip_parens(expression) {
        Expression::RegExpLiteral(literal) => literal.regex.flags.intersects(RegExpFlags::G | RegExpFlags::Y),
        Expression::NewExpression(new) => is_global(ctx, &new.callee, "RegExp") && stateful_flags(&new.arguments),
        Expression::Identifier(identifier) => const_initialiser(ctx, identifier)
            .is_some_and(|init| matches!(strip_parens(init), Expression::RegExpLiteral(_) | Expression::NewExpression(_)) && is_stateful_regex(ctx, init)),
        _ => false,
    }
}

/// Whether the flags argument of `RegExp(pattern, flags)` holds `g` or `y`.
fn stateful_flags(arguments: &[Argument<'_>]) -> bool {
    matches!(arguments.get(1), Some(Argument::StringLiteral(flags)) if flags.value.contains(['g', 'y']))
}

/// The initialiser of the `const` declaration `identifier` refers to, when
/// this file declares it.
pub fn const_initialiser<'a>(ctx: &Context<'_, 'a>, identifier: &IdentifierReference<'a>) -> Option<&'a Expression<'a>> {
    let reference = identifier.reference_id.get()?;
    let symbol = ctx.semantic.scoping().get_reference(reference).symbol_id()?;
    let declaration = ctx.semantic.symbol_declaration(symbol);

    let AstKind::VariableDeclarator(declarator) = declaration.kind() else {
        return None;
    };

    let AstKind::VariableDeclaration(list) = ctx.semantic.nodes().parent_kind(declaration.id()) else {
        return None;
    };

    if list.kind != VariableDeclarationKind::Const {
        return None;
    }

    declarator.init.as_ref()
}

/// The receiver a call chain starts from: `db` in `db.select().from(t)`, and
/// `db` in `this.db.insert(t).values(v)`.
pub fn receiver<'a>(mut expression: &'a Expression<'a>) -> Option<&'a str> {
    loop {
        match strip_parens(expression) {
            Expression::Identifier(identifier) => return Some(identifier.name.as_str()),
            Expression::StaticMemberExpression(member) => {
                if matches!(strip_parens(&member.object), Expression::ThisExpression(_)) {
                    return Some(member.property.name.as_str());
                }

                expression = &member.object;
            }
            Expression::CallExpression(call) => expression = &call.callee,
            _ => return None,
        }
    }
}

/// Whether a call to one of `methods` sits anywhere below `expression` in its
/// chain: `insert` in `db.insert(t).values(v)`.
pub fn chain_calls(mut expression: &Expression<'_>, methods: &[&str]) -> bool {
    loop {
        match strip_parens(expression) {
            Expression::CallExpression(call) => {
                if member(&call.callee).is_some_and(|(_, name)| methods.contains(&name)) {
                    return true;
                }

                expression = &call.callee;
            }
            Expression::StaticMemberExpression(member) => expression = &member.object,
            Expression::ComputedMemberExpression(member) => expression = &member.object,
            _ => return false,
        }
    }
}
