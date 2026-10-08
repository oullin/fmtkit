//! Which sibling pairs need a blank line between them, and how class members
//! group for reordering. Type names in comments are the ESTree ones v1 used.

use oxc_ast::ast::{
    AccessorPropertyType, ArrowFunctionExpression, AwaitExpression, ClassElement, Declaration, Expression, ExportDefaultDeclarationKind, Function, FunctionType,
    MethodDefinitionKind, Statement, VariableDeclarationKind,
};
use oxc_ast_visit::Visit;
use oxc_ast_visit::walk::walk_await_expression;
use oxc_syntax::scope::ScopeFlags;

use super::lists::Item;

/// The Vue composition primitives whose statements open a new paragraph.
const VUE_PRIMITIVES: [&str; 21] = [
    "computed",
    "nextTick",
    "onActivated",
    "onBeforeMount",
    "onBeforeUnmount",
    "onBeforeUpdate",
    "onDeactivated",
    "onErrorCaptured",
    "onMounted",
    "onRenderTracked",
    "onRenderTriggered",
    "onServerPrefetch",
    "onUnmounted",
    "onUpdated",
    "reactive",
    "readonly",
    "ref",
    "shallowReactive",
    "shallowRef",
    "watch",
    "watchEffect",
];

/// Where a class member sorts: properties, then constructors, then methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MemberKind {
    Property,
    Constructor,
    Method,
}

pub(crate) fn classify(member: &ClassElement<'_>) -> MemberKind {
    if is_property_member(member) {
        return MemberKind::Property;
    }

    match member {
        ClassElement::MethodDefinition(method) if method.kind == MethodDefinitionKind::Constructor => MemberKind::Constructor,
        _ => MemberKind::Method,
    }
}

/// `PropertyDefinition`, `TSAbstractPropertyDefinition`, `AccessorProperty`,
/// `TSIndexSignature`, and `StaticBlock`. An abstract accessor is none of these.
fn is_property_member(member: &ClassElement<'_>) -> bool {
    match member {
        ClassElement::PropertyDefinition(_) | ClassElement::TSIndexSignature(_) | ClassElement::StaticBlock(_) => true,
        ClassElement::AccessorProperty(accessor) => accessor.r#type == AccessorPropertyType::AccessorProperty,
        ClassElement::MethodDefinition(_) => false,
    }
}

fn is_method_member(item: Item<'_>) -> bool {
    matches!(item, Item::Member(ClassElement::MethodDefinition(_)))
}

fn is_property_item(item: Item<'_>) -> bool {
    matches!(item, Item::Member(member) if is_property_member(member))
}

/// Whether a blank line belongs between two adjacent siblings.
pub(crate) fn needs_blank_line(previous: Item<'_>, next: Item<'_>) -> bool {
    // A `case` with no consequent is one label of a fallthrough group; oxlint's
    // `no-fallthrough` reports a label split from the one below it.
    if let (Item::Case(case), Item::Case(_)) = (previous, next)
        && case.consequent.is_empty()
    {
        return false;
    }

    if contains_await(previous) || contains_await(next) || needs_blank_line_above(next) {
        return true;
    }

    if is_binding_call(next) && !is_binding_call(previous) {
        return true;
    }

    if is_loop(next) {
        return !is_structured(previous);
    }

    if ((is_method_member(previous) || is_property_item(previous)) && is_method_member(next)) || is_type_declaration(previous) {
        return true;
    }

    if previous.is_import() && !next.is_import() {
        return true;
    }

    if previous.is_const() != next.is_const() {
        return true;
    }

    let is_let = |item: Item<'_>| item.declaration_kind() == Some(VariableDeclarationKind::Let);

    if is_let(previous) != is_let(next) {
        return true;
    }

    if previous.declaration_kind().is_some() && next.declaration_kind().is_none() {
        return true;
    }

    matches!(
        previous.statement(),
        Some(
            Statement::IfStatement(_)
                | Statement::ForStatement(_)
                | Statement::ForInStatement(_)
                | Statement::ForOfStatement(_)
                | Statement::WhileStatement(_)
                | Statement::DoWhileStatement(_)
                | Statement::SwitchStatement(_)
                | Statement::TryStatement(_)
        )
    )
}

fn needs_blank_line_above(next: Item<'_>) -> bool {
    if matches!(next, Item::Case(_)) {
        return true;
    }

    let Some(statement) = next.statement() else {
        return false;
    };

    if is_vue_primitive_statement(statement) {
        return true;
    }

    match statement {
        Statement::ReturnStatement(_)
        | Statement::SwitchStatement(_)
        | Statement::ClassDeclaration(_)
        | Statement::TSEnumDeclaration(_)
        | Statement::TSExternalModuleDeclaration(_)
        | Statement::TSNamespaceDeclaration(_)
        | Statement::TSGlobalDeclaration(_)
        | Statement::ExportDeclaration(_)
        | Statement::ExportDefaultDeclaration(_) => true,
        Statement::FunctionDeclaration(function) => is_function_declaration(function),
        _ => false,
    }
}

/// ESTree `FunctionDeclaration`: not a bodiless `TSDeclareFunction`.
fn is_function_declaration(function: &Function<'_>) -> bool {
    function.r#type == FunctionType::FunctionDeclaration
}

fn is_type_declaration_kind(declaration: &Declaration<'_>) -> bool {
    matches!(
        declaration,
        Declaration::TSTypeAliasDeclaration(_)
            | Declaration::TSInterfaceDeclaration(_)
            | Declaration::TSEnumDeclaration(_)
            | Declaration::TSExternalModuleDeclaration(_)
            | Declaration::TSNamespaceDeclaration(_)
            | Declaration::TSGlobalDeclaration(_)
    )
}

fn is_type_declaration(previous: Item<'_>) -> bool {
    match previous.statement() {
        Some(Statement::ExportDeclaration(export)) => is_type_declaration_kind(&export.declaration),
        Some(statement) => statement.as_declaration().is_some_and(is_type_declaration_kind),
        None => false,
    }
}

fn is_loop(item: Item<'_>) -> bool {
    matches!(
        item.statement(),
        Some(Statement::ForStatement(_) | Statement::ForInStatement(_) | Statement::ForOfStatement(_) | Statement::WhileStatement(_) | Statement::DoWhileStatement(_))
    )
}

/// A statement that already ends in a block, directly or as an exported declaration.
fn is_structured(previous: Item<'_>) -> bool {
    match previous.statement() {
        Some(
            Statement::ClassDeclaration(_)
            | Statement::DoWhileStatement(_)
            | Statement::ForInStatement(_)
            | Statement::ForOfStatement(_)
            | Statement::ForStatement(_)
            | Statement::IfStatement(_)
            | Statement::SwitchStatement(_)
            | Statement::TryStatement(_)
            | Statement::WhileStatement(_),
        ) => true,
        Some(Statement::FunctionDeclaration(function)) => is_function_declaration(function),
        Some(Statement::ExportDeclaration(export)) => match &export.declaration {
            Declaration::ClassDeclaration(_) => true,
            Declaration::FunctionDeclaration(function) => is_function_declaration(function),
            _ => false,
        },
        Some(Statement::ExportDefaultDeclaration(export)) => match &export.declaration {
            ExportDefaultDeclarationKind::ClassDeclaration(_) => true,
            ExportDefaultDeclarationKind::FunctionDeclaration(function) => is_function_declaration(function),
            _ => false,
        },
        _ => false,
    }
}

/// `bindings.x(...)` or `<object>.bindings.x(...)` as an expression statement.
fn is_binding_call(item: Item<'_>) -> bool {
    let Some(Statement::ExpressionStatement(statement)) = item.statement() else {
        return false;
    };

    let Expression::CallExpression(call) = &statement.expression else {
        return false;
    };

    let Expression::StaticMemberExpression(callee) = &call.callee else {
        return false;
    };

    match &callee.object {
        Expression::Identifier(receiver) => receiver.name == "bindings",
        Expression::StaticMemberExpression(receiver) => receiver.property.name == "bindings",
        _ => false,
    }
}

fn is_vue_primitive_call(expression: &Expression<'_>) -> bool {
    let Expression::CallExpression(call) = expression else {
        return false;
    };

    matches!(&call.callee, Expression::Identifier(callee) if VUE_PRIMITIVES.contains(&callee.name.as_str()))
}

fn is_vue_primitive_statement(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::ExpressionStatement(statement) => is_vue_primitive_call(&statement.expression),
        Statement::VariableDeclaration(declaration) if declaration.kind == VariableDeclarationKind::Const => {
            declaration.declarations.iter().any(|declarator| declarator.init.as_ref().is_some_and(is_vue_primitive_call))
        }
        _ => false,
    }
}

/// Whether an `await` appears in the item outside nested functions.
fn contains_await(item: Item<'_>) -> bool {
    let mut finder = AwaitFinder { found: false };

    match item {
        Item::Directive(_) => {}
        Item::Statement(statement) => finder.visit_statement(statement),
        Item::Member(member) => finder.visit_class_element(member),
        Item::Case(case) => finder.visit_switch_case(case),
    }

    finder.found
}

struct AwaitFinder {
    found: bool,
}

impl<'a> Visit<'a> for AwaitFinder {
    fn visit_await_expression(&mut self, it: &AwaitExpression<'a>) {
        self.found = true;
        walk_await_expression(self, it);
    }

    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}
}
