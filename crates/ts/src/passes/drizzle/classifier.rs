//! Which calls are Drizzle queries and which of their arguments are worth
//! laying out one per line.

use oxc_ast::ast::{Argument, ArrayExpression, ArrayExpressionElement, CallExpression, Expression, ObjectExpression, ObjectPropertyKind, PropertyKey};

use super::imports::DrizzleImports;
use super::vocabulary;
use crate::syntax::{Chained, static_property_name};

/// Calls classified against one module's Drizzle imports.
pub(crate) struct Classifier<'i, 'a> {
    pub imports: &'i DrizzleImports<'a>,
}

const WHERE_METHODS: [&str; 3] = ["where", "having", "$count"];
const JOIN_METHODS: [&str; 5] = ["leftJoin", "rightJoin", "innerJoin", "fullJoin", "crossJoin"];
const MUTATION_METHODS: [&str; 5] = ["onConflictDoNothing", "onConflictDoUpdate", "returning", "set", "values"];
const STRUCTURAL_METHODS: [&str; 7] = ["as", "except", "groupBy", "intersect", "orderBy", "union", "unionAll"];

impl<'a> Classifier<'_, 'a> {
    /// The Drizzle export an identifier or `namespace.name` callee refers to.
    pub(crate) fn callee_name(&self, callee: Chained<'a>) -> Option<&'a str> {
        match callee {
            Chained::Other(Expression::Identifier(identifier)) => self.imports.local_import(identifier.name.as_str()),
            Chained::Member(member) => {
                let property = static_property_name(member)?;
                let Expression::Identifier(object) = member.object() else { return None };

                self.imports.has_namespace(object.name.as_str()).then_some(property)
            }
            _ => None,
        }
    }

    pub(crate) fn is_method_call(&self, call: &'a CallExpression<'a>) -> bool {
        let Chained::Member(member) = Chained::of(&call.callee) else {
            return false;
        };

        static_property_name(member).is_some_and(vocabulary::is_format_method) && self.is_receiver(member.object())
    }

    pub(crate) fn is_relational_query(&self, call: &'a CallExpression<'a>) -> bool {
        let Chained::Member(member) = Chained::of(&call.callee) else {
            return false;
        };

        matches!(static_property_name(member), Some("findMany" | "findFirst")) && has_query_member(member.object()) && self.is_receiver(member.object())
    }

    pub(crate) fn is_helper_call(&self, call: &'a CallExpression<'a>) -> bool {
        self.callee_name(Chained::of(&call.callee)).is_some_and(vocabulary::is_helper)
    }

    pub(crate) fn is_set_operation(&self, call: &'a CallExpression<'a>) -> bool {
        self.callee_name(Chained::of(&call.callee)).is_some_and(vocabulary::is_set_operation)
    }

    pub(crate) fn formats_method_arguments(&self, call: &'a CallExpression<'a>) -> bool {
        let arguments = &call.arguments;

        if arguments.is_empty() {
            return false;
        }

        if self.is_relational_query(call) {
            return arguments.iter().any(|argument| matches!(argument, Argument::ObjectExpression(object) if formats_object(object)));
        }

        let Chained::Member(member) = Chained::of(&call.callee) else {
            return false;
        };

        let Some(name) = static_property_name(member) else {
            return false;
        };

        let complex = |argument: &'a Argument<'a>| self.is_complex_argument(argument);

        if WHERE_METHODS.contains(&name) || MUTATION_METHODS.contains(&name) {
            return arguments.iter().any(complex);
        }

        if JOIN_METHODS.contains(&name) {
            return arguments.len() > 1 && arguments.iter().skip(1).any(complex);
        }

        if STRUCTURAL_METHODS.contains(&name) {
            return arguments.len() > 1 || arguments.iter().any(|argument| self.is_structural_argument(argument));
        }

        arguments.len() > 1 && arguments.iter().any(complex)
    }

    fn is_receiver(&self, expression: &'a Expression<'a>) -> bool {
        match Chained::of(expression) {
            Chained::Other(Expression::Identifier(identifier)) => vocabulary::is_receiver(identifier.name.as_str()),
            Chained::Member(member) => self.is_receiver(member.object()),
            Chained::Call(call) => match Chained::of(&call.callee) {
                callee @ Chained::Other(Expression::Identifier(_)) => self.callee_name(callee).is_some_and(vocabulary::is_set_operation),
                Chained::Member(member) => self.is_receiver(member.object()),
                _ => false,
            },
            _ => false,
        }
    }

    fn is_complex_argument(&self, argument: &'a Argument<'a>) -> bool {
        match argument {
            Argument::ObjectExpression(object) => formats_object(object),
            Argument::ArrayExpression(array) => formats_array(array),
            Argument::CallExpression(call) => self.is_helper_call(call) || self.is_set_operation(call) || self.is_method_call(call),
            _ => false,
        }
    }

    fn is_structural_argument(&self, argument: &'a Argument<'a>) -> bool {
        match argument {
            Argument::ObjectExpression(_) | Argument::ArrayExpression(_) => true,
            Argument::CallExpression(call) => self.is_set_operation(call) || self.is_method_call(call),
            _ => false,
        }
    }
}

fn has_query_member(expression: &Expression<'_>) -> bool {
    match Chained::of(expression) {
        Chained::Member(member) => static_property_name(member) == Some("query") || has_query_member(member.object()),
        Chained::Call(call) => has_query_member(&call.callee),
        _ => false,
    }
}

/// The ESTree `Identifier` name of a property key, computed or not.
pub(crate) fn key_name<'a>(key: &'a PropertyKey<'a>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Some(identifier.name.as_str()),
        PropertyKey::Identifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

pub(crate) fn formats_object(object: &ObjectExpression<'_>) -> bool {
    if object.properties.len() > 1 {
        return true;
    }

    object.properties.iter().any(|property| {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return true;
        };

        let structured = matches!(property.value, Expression::ObjectExpression(_) | Expression::ArrayExpression(_));
        let keyed = key_name(&property.key).is_some_and(vocabulary::formats_object_key) && matches!(property.value, Expression::CallExpression(_));

        structured || keyed
    })
}

pub(crate) fn formats_array(array: &ArrayExpression<'_>) -> bool {
    array.elements.len() > 1 || array.elements.iter().any(|element| matches!(element, ArrayExpressionElement::ObjectExpression(_) | ArrayExpressionElement::CallExpression(_)))
}
