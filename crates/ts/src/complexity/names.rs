//! Reporting names: the text a declaration gives the function or class it
//! holds, rendered the way v1 rendered the ESTree node.
//!
//! v1 read `name` (an identifier), then a string `value` (a string literal),
//! joined member paths with dots and prefixed private names with `#`. Every
//! other node rendered as `''`, so a parenthesised, `as`-cast, numeric,
//! template, or optional-chain name leaves the function anonymous.

use oxc_ast::ast::{AssignmentTarget, BindingPattern, Expression, MethodDefinitionKind, PropertyKey, PropertyKind};

/// The reporting name given to a function no declaration names.
pub const ANONYMOUS: &str = "<anonymous>";

/// Where a declaration keeps the name of the function or class it holds.
#[derive(Clone, Copy)]
pub enum NameSource<'s, 'a> {
    /// `const name = …` (a `VariableDeclarator` id).
    Binding(&'s BindingPattern<'a>),
    /// `target = …` (an `AssignmentExpression` left side).
    Target(&'s AssignmentTarget<'a>),
    /// An object property, or a class method or field when `qualified`.
    Key { key: &'s PropertyKey<'a>, accessor: &'static str, qualified: bool },
    /// The child of `export default`.
    Default,
}

impl NameSource<'_, '_> {
    /// Render the name, qualifying class members with `class_name`. Returns
    /// `''` when the declaration names nothing usable.
    pub fn render(self, class_name: &str) -> String {
        let mut out = String::new();

        match self {
            Self::Binding(BindingPattern::BindingIdentifier(id)) => out.push_str(id.name.as_str()),
            Self::Binding(_) => {}
            Self::Target(target) => write_target(&mut out, target),
            Self::Key { key, accessor, qualified } => {
                out.push_str(accessor);
                write_key(&mut out, key);

                if qualified && !out.is_empty() && !class_name.is_empty() {
                    out.insert(0, '.');
                    out.insert_str(0, class_name);
                }
            }
            Self::Default => out.push_str("default"),
        }

        out
    }
}

/// The `get `/`set ` prefix that keeps an accessor pair on two keys.
pub const fn method_accessor(kind: MethodDefinitionKind) -> &'static str {
    match kind {
        MethodDefinitionKind::Get => "get ",
        MethodDefinitionKind::Set => "set ",
        MethodDefinitionKind::Constructor | MethodDefinitionKind::Method => "",
    }
}

/// The `get `/`set ` prefix for an object-literal accessor.
pub const fn property_accessor(kind: PropertyKind) -> &'static str {
    match kind {
        PropertyKind::Get => "get ",
        PropertyKind::Set => "set ",
        PropertyKind::Init => "",
    }
}

fn write_key(out: &mut String, key: &PropertyKey<'_>) {
    match key {
        PropertyKey::StaticIdentifier(id) => out.push_str(id.name.as_str()),
        PropertyKey::PrivateIdentifier(id) => {
            out.push('#');
            out.push_str(id.name.as_str());
        }
        PropertyKey::Identifier(id) => out.push_str(id.name.as_str()),
        PropertyKey::StringLiteral(lit) => out.push_str(lit.value.as_str()),
        PropertyKey::StaticMemberExpression(member) => write_member(out, &member.object, |out| out.push_str(member.property.name.as_str())),
        PropertyKey::ComputedMemberExpression(member) => write_member(out, &member.object, |out| write_expression(out, &member.expression)),
        PropertyKey::PrivateFieldExpression(member) => write_member(out, &member.object, |out| write_private(out, member.field.name.as_str())),
        _ => {}
    }
}

fn write_target(out: &mut String, target: &AssignmentTarget<'_>) {
    match target {
        AssignmentTarget::AssignmentTargetIdentifier(id) => out.push_str(id.name.as_str()),
        AssignmentTarget::StaticMemberExpression(member) => write_member(out, &member.object, |out| out.push_str(member.property.name.as_str())),
        AssignmentTarget::ComputedMemberExpression(member) => write_member(out, &member.object, |out| write_expression(out, &member.expression)),
        AssignmentTarget::PrivateFieldExpression(member) => write_member(out, &member.object, |out| write_private(out, member.field.name.as_str())),
        _ => {}
    }
}

fn write_expression(out: &mut String, expression: &Expression<'_>) {
    match expression {
        Expression::Identifier(id) => out.push_str(id.name.as_str()),
        Expression::StringLiteral(lit) => out.push_str(lit.value.as_str()),
        Expression::StaticMemberExpression(member) => write_member(out, &member.object, |out| out.push_str(member.property.name.as_str())),
        Expression::ComputedMemberExpression(member) => write_member(out, &member.object, |out| write_expression(out, &member.expression)),
        Expression::PrivateFieldExpression(member) => write_member(out, &member.object, |out| write_private(out, member.field.name.as_str())),
        _ => {}
    }
}

fn write_private(out: &mut String, name: &str) {
    out.push('#');
    out.push_str(name);
}

/// Write `object.property`, dropping either side that renders empty (v1's
/// `[object, property].filter(Boolean).join('.')`).
fn write_member(out: &mut String, object: &Expression<'_>, property: impl FnOnce(&mut String)) {
    let start = out.len();

    write_expression(out, object);

    let dotted = out.len() > start;

    if dotted {
        out.push('.');
    }

    let before = out.len();

    property(out);

    if dotted && out.len() == before {
        out.pop();
    }
}
