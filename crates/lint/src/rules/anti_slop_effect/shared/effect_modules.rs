//! Upstream's `effect/shared/effect-modules.ts`: the local names a file binds
//! to the Effect modules, readers for `Context.Service` classes and their
//! static members, and the subtree walks the service rules share.

use std::rc::Rc;

use oxc_ast::AstKind;
use oxc_ast::ast::{
    Argument, ArrowFunctionBody, ArrowFunctionExpression, CallExpression, Class, ClassElement, Expression, Function, FunctionType, IdentifierReference,
    ImportDeclarationSpecifier, MethodDefinitionType, ObjectExpression, ObjectProperty, ObjectPropertyKind, Program, PropertyDefinitionType, PropertyKey,
    ReturnStatement, Statement, StaticMemberExpression,
};
use oxc_semantic::{AstNode, AstNodes, NodeId, Semantic, SymbolId};
use oxc_span::{GetSpan, Span};

use super::super::super::Context;
use super::super::super::anti_slop::shared::{ancestors, strip_parens};

/// An Effect module whose members the Effect rules recognize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Module {
    Context,
    Effect,
    Layer,
    ManagedRuntime,
}

impl Module {
    pub const ALL: [Self; 4] = [Self::Context, Self::Effect, Self::Layer, Self::ManagedRuntime];

    fn named(name: &str) -> Option<Self> {
        match name {
            "Context" => Some(Self::Context),
            "Effect" => Some(Self::Effect),
            "Layer" => Some(Self::Layer),
            "ManagedRuntime" => Some(Self::ManagedRuntime),
            _ => None,
        }
    }
}

/// The local names bound to each Effect module by the file's imports:
/// `import { Effect } from "effect"` and `import * as Effect from "effect/Effect"`.
#[derive(Debug, Default)]
pub struct Modules {
    names: Vec<(Module, Box<str>)>,
}

impl Modules {
    /// The file's names, collected once per file.
    pub fn of(ctx: &mut Context<'_, '_>) -> Rc<Self> {
        ctx.shared(|semantic| Self::collect(semantic.nodes().program()))
    }

    fn collect(program: &Program<'_>) -> Self {
        let mut names = Vec::new();

        for statement in &program.body {
            let Statement::ImportDeclaration(import) = statement else {
                continue;
            };
            let Some(specifiers) = &import.specifiers else {
                continue;
            };
            let source = import.source.value.as_str();

            if source == "effect" {
                for specifier in specifiers {
                    if let ImportDeclarationSpecifier::ImportSpecifier(specifier) = specifier
                        && let Some(module) = Module::named(&specifier.imported.name())
                    {
                        names.push((module, specifier.local.name.as_str().into()));
                    }
                }

                continue;
            }

            let Some(module) = source.strip_prefix("effect/").and_then(Module::named) else {
                continue;
            };

            for specifier in specifiers {
                match specifier {
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(it) => names.push((module, it.local.name.as_str().into())),
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(it) => names.push((module, it.local.name.as_str().into())),
                    ImportDeclarationSpecifier::ImportSpecifier(_) => {}
                }
            }
        }

        Self { names }
    }

    pub fn has(&self, module: Module, local: &str) -> bool {
        self.names.iter().any(|(bound, name)| *bound == module && &**name == local)
    }
}

/// `expression` without parentheses, type assertions, non-null assertions
/// and instantiation expressions, as upstream's `unwrapExpression`.
pub fn unwrap<'b, 'a>(mut expression: &'b Expression<'a>) -> &'b Expression<'a> {
    loop {
        expression = match expression {
            Expression::ParenthesizedExpression(inner) => &inner.expression,
            Expression::TSAsExpression(inner) => &inner.expression,
            Expression::TSSatisfiesExpression(inner) => &inner.expression,
            Expression::TSTypeAssertion(inner) => &inner.expression,
            Expression::TSNonNullExpression(inner) => &inner.expression,
            Expression::TSInstantiationExpression(inner) => &inner.expression,
            _ => return expression,
        };
    }
}

/// Whether ESTree shows this kind as a wrapper [`unwrap`] looks through.
pub fn is_wrapper(kind: AstKind<'_>) -> bool {
    matches!(
        kind,
        AstKind::ParenthesizedExpression(_)
            | AstKind::TSAsExpression(_)
            | AstKind::TSSatisfiesExpression(_)
            | AstKind::TSTypeAssertion(_)
            | AstKind::TSNonNullExpression(_)
            | AstKind::TSInstantiationExpression(_)
    )
}

/// The member name when `expression` is `<module>.<member>` for one of the
/// module's local names.
pub fn member_name<'a>(expression: &Expression<'a>, modules: &Modules, module: Module) -> Option<&'a str> {
    let mut expression = strip_parens(expression);

    if let Expression::TSInstantiationExpression(inner) = expression {
        expression = strip_parens(&inner.expression);
    }

    let Expression::StaticMemberExpression(member) = expression else {
        return None;
    };
    let Expression::Identifier(object) = strip_parens(&member.object) else {
        return None;
    };

    modules.has(module, object.name.as_str()).then_some(member.property.name.as_str())
}

/// The member name when `expression` is a call of `<module>.<member>(...)`.
pub fn call_name<'a>(expression: &Expression<'a>, modules: &Modules, module: Module) -> Option<&'a str> {
    match strip_parens(expression) {
        Expression::CallExpression(call) => callee_name(call, modules, module),
        _ => None,
    }
}

/// The member name when `call` calls `<module>.<member>`.
pub fn callee_name<'a>(call: &CallExpression<'a>, modules: &Modules, module: Module) -> Option<&'a str> {
    member_name(unwrap(&call.callee), modules, module)
}

/// Whether `call` passes `span` (a node without its parentheses) as an argument.
pub fn has_argument(call: &CallExpression<'_>, span: Span) -> bool {
    call.arguments.iter().any(|argument| argument.as_expression().is_some_and(|argument| strip_parens(argument).span() == span))
}

/// The parent of `id` as ESTree has it, or `None` at the root.
pub fn estree_parent<'s, 'a>(nodes: &'s AstNodes<'a>, id: NodeId) -> Option<&'s AstNode<'a>> {
    ancestors(nodes, id).next()
}

/// The parts of `class X extends Context.Service<...>()("key", options) {}`.
pub struct Service<'a> {
    pub class: &'a Class<'a>,
    pub name: Option<&'a str>,
    pub options: Option<&'a ObjectExpression<'a>>,
}

/// Read a `Context.Service` class, or `None` when the class is not one.
pub fn service_class<'a>(class: &'a Class<'a>, modules: &Modules) -> Option<Service<'a>> {
    let heritage = class.heritage.as_ref()?;
    let Expression::CallExpression(super_call) = unwrap(&heritage.expression) else {
        return None;
    };
    let Expression::CallExpression(factory) = unwrap(&super_call.callee) else {
        return None;
    };

    if member_name(unwrap(&factory.callee), modules, Module::Context) != Some("Service") {
        return None;
    }

    let options = match super_call.arguments.get(1).and_then(Argument::as_expression).map(unwrap) {
        Some(Expression::ObjectExpression(options)) => Some(&**options),
        _ => None,
    };

    Some(Service { class, name: class.id.as_ref().map(|id| id.name.as_str()), options })
}

/// The innermost class whose body contains `id`.
pub fn enclosing_class<'a>(nodes: &AstNodes<'a>, id: NodeId) -> Option<&'a Class<'a>> {
    let body = ancestors(nodes, id).find(|node| matches!(node.kind(), AstKind::ClassBody(_)))?;

    match nodes.parent_kind(body.id()) {
        AstKind::Class(class) => Some(class),
        _ => None,
    }
}

/// Whether `expression` names the service class: `this`, or its class name.
fn is_class_reference(expression: &Expression<'_>, service: &Service<'_>) -> bool {
    match unwrap(expression) {
        Expression::ThisExpression(_) => true,
        Expression::Identifier(identifier) => service.name == Some(identifier.name.as_str()),
        _ => false,
    }
}

/// The member name when `expression` is `this.<member>` or `<ClassName>.<member>`.
pub fn class_member_name<'a>(expression: &Expression<'a>, service: &Service<'_>) -> Option<&'a str> {
    match unwrap(expression) {
        Expression::StaticMemberExpression(member) => static_member_of_class(member, service),
        _ => None,
    }
}

/// The member name when `member` is `this.<member>` or `<ClassName>.<member>`.
pub fn static_member_of_class<'a>(member: &StaticMemberExpression<'a>, service: &Service<'_>) -> Option<&'a str> {
    is_class_reference(&member.object, service).then_some(member.property.name.as_str())
}

/// The name of a non-computed property key, as upstream's `keyName`.
pub fn key_name<'a>(key: &PropertyKey<'a>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Some(identifier.name.as_str()),
        PropertyKey::PrivateIdentifier(identifier) => Some(identifier.name.as_str()),
        PropertyKey::StringLiteral(literal) => Some(literal.value.as_str()),
        _ => None,
    }
}

/// The first non-computed property of `object` named `name`.
fn object_property<'a>(object: &'a ObjectExpression<'a>, name: &str) -> Option<&'a ObjectProperty<'a>> {
    object.properties.iter().find_map(|property| match property {
        ObjectPropertyKind::ObjectProperty(property) if !property.computed && key_name(&property.key) == Some(name) => Some(&**property),
        _ => None,
    })
}

/// A property initializer or a method's function.
#[derive(Clone, Copy)]
pub enum Value<'a> {
    Expression(&'a Expression<'a>),
    Function(&'a Function<'a>),
}

impl Value<'_> {
    pub fn node_id(self) -> NodeId {
        match self {
            Self::Expression(expression) => expression.node_id(),
            Self::Function(function) => function.node_id(),
        }
    }
}

/// A static class property or method, with its name.
pub struct StaticMember<'a> {
    pub name: &'a str,
    pub span: Span,
    pub value: Option<Value<'a>>,
}

/// Every non-computed static property and method of a class, in order.
pub fn static_members<'a>(class: &'a Class<'a>) -> Vec<StaticMember<'a>> {
    class
        .body
        .body
        .iter()
        .filter_map(|element| match element {
            ClassElement::PropertyDefinition(property)
                if property.r#type == PropertyDefinitionType::PropertyDefinition && property.r#static && !property.computed =>
            {
                Some(StaticMember { name: key_name(&property.key)?, span: property.span, value: property.value.as_ref().map(Value::Expression) })
            }
            ClassElement::MethodDefinition(method) if method.r#type == MethodDefinitionType::MethodDefinition && method.r#static && !method.computed => {
                Some(StaticMember { name: key_name(&method.key)?, span: method.span, value: Some(Value::Function(&method.value)) })
            }
            _ => None,
        })
        .collect()
}

/// The values that construct the service: the options' `make` and a static `make`.
pub fn service_makes<'a>(service: &Service<'a>) -> Vec<Value<'a>> {
    let mut makes: Vec<Value<'a>> =
        service.options.and_then(|options| object_property(options, "make")).map(|make| Value::Expression(&make.value)).into_iter().collect();

    makes.extend(static_members(service.class).into_iter().filter(|member| member.name == "make").filter_map(|member| member.value));
    makes
}

/// Whether `name` names a layer member: `layer`, `layerFoo`, `layer_2`.
pub fn is_layer_member_name(name: &str) -> bool {
    name.strip_prefix("layer").is_some_and(|rest| rest.is_empty() || rest.starts_with(|c: char| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'))
}

/// What a walk does after visiting a node.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Visit the node's children.
    Into,
    /// Skip the node's children.
    Over,
    /// End the walk.
    Stop,
}

/// Visit `root` and every node under it in source order, as upstream's `walk`.
///
/// Node ids are assigned in pre-order, so a subtree is the run of ids after
/// its root whose parents lie inside it.
pub fn walk<'a>(nodes: &AstNodes<'a>, root: NodeId, mut visit: impl FnMut(&AstNode<'a>) -> Step) {
    match visit(nodes.get_node(root)) {
        Step::Into => {}
        Step::Over | Step::Stop => return,
    }

    let mut skipped: Option<NodeId> = None;

    for index in root.index() + 1..nodes.len() {
        let id = NodeId::new(index);
        let parent = nodes.parent_id(id);

        if parent < root {
            break;
        }

        if let Some(skipped_root) = skipped {
            if parent >= skipped_root {
                continue;
            }

            skipped = None;
        }

        match visit(nodes.get_node(id)) {
            Step::Into => {}
            Step::Over => skipped = Some(id),
            Step::Stop => return,
        }
    }
}

/// Whether `root` or any node under it satisfies `predicate`.
pub fn contains<'a>(nodes: &AstNodes<'a>, root: NodeId, mut predicate: impl FnMut(&AstNode<'a>) -> bool) -> bool {
    let mut found = false;

    walk(nodes, root, |node| {
        found = predicate(node);

        if found { Step::Stop } else { Step::Into }
    });

    found
}

/// A function expression, arrow, or declaration.
#[derive(Clone, Copy)]
pub enum FunctionNode<'a> {
    Arrow(&'a ArrowFunctionExpression<'a>),
    Function(&'a Function<'a>),
}

impl<'a> FunctionNode<'a> {
    /// An arrow or a function expression, as upstream's `isFunction` on an expression.
    pub fn of(expression: &'a Expression<'a>) -> Option<Self> {
        match strip_parens(expression) {
            Expression::ArrowFunctionExpression(arrow) => Some(Self::Arrow(arrow)),
            Expression::FunctionExpression(function) => Some(Self::Function(function)),
            _ => None,
        }
    }

    /// An arrow, a function expression or a function declaration.
    pub fn of_kind(kind: AstKind<'a>) -> Option<Self> {
        match kind {
            AstKind::ArrowFunctionExpression(arrow) => Some(Self::Arrow(arrow)),
            AstKind::Function(function) if matches!(function.r#type, FunctionType::FunctionDeclaration | FunctionType::FunctionExpression) => {
                Some(Self::Function(function))
            }
            _ => None,
        }
    }
}

/// Whether ESTree shows `kind` as a function: an arrow, a function expression
/// or a function declaration.
pub fn is_function(kind: AstKind<'_>) -> bool {
    FunctionNode::of_kind(kind).is_some()
}

/// The return statements under `body`, without descending into nested functions.
pub fn own_returns<'a>(nodes: &AstNodes<'a>, body: NodeId) -> Vec<&'a ReturnStatement<'a>> {
    let mut returns = Vec::new();

    walk(nodes, body, |node| match node.kind() {
        _ if node.id() != body && is_function(node.kind()) => Step::Over,
        AstKind::ReturnStatement(statement) => {
            returns.push(statement);
            Step::Into
        }
        _ => Step::Into,
    });

    returns
}

/// What a function produces: its expression body or the arguments of its own returns.
pub fn returned_expressions<'a>(nodes: &AstNodes<'a>, function: FunctionNode<'a>) -> Vec<&'a Expression<'a>> {
    let body = match function {
        FunctionNode::Arrow(arrow) => match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => &**body,
            expression => return expression.as_expression().into_iter().collect(),
        },
        FunctionNode::Function(function) => match &function.body {
            Some(body) => &**body,
            None => return Vec::new(),
        },
    };

    own_returns(nodes, body.node_id()).into_iter().filter_map(|statement| statement.argument.as_ref()).collect()
}

/// For `x.pipe(...)`, the receiver `x`.
pub fn pipe_receiver<'b, 'a>(expression: &'b Expression<'a>) -> Option<&'b Expression<'a>> {
    match strip_parens(expression) {
        Expression::CallExpression(call) => call_pipe_receiver(call),
        _ => None,
    }
}

/// For a call `x.pipe(...)`, the receiver `x`.
pub fn call_pipe_receiver<'b, 'a>(call: &'b CallExpression<'a>) -> Option<&'b Expression<'a>> {
    match unwrap(&call.callee) {
        Expression::StaticMemberExpression(member) if member.property.name == "pipe" => Some(unwrap(&member.object)),
        _ => None,
    }
}

/// The steps that run before `call` when it is an argument of a pipeline-like
/// call such as `x.pipe(a, call)`, `pipe(x, a, call)` or
/// `Effect.fn("name")(body, a, call)`, including earlier `.pipe(...)`
/// receivers. Spread arguments are left out: no step test matches them.
pub fn preceding_steps<'a>(nodes: &AstNodes<'a>, call: &AstNode<'a>) -> Vec<&'a Expression<'a>> {
    let Some(AstKind::CallExpression(parent)) = estree_parent(nodes, call.id()).map(AstNode::kind) else {
        return Vec::new();
    };
    let span = call.kind().span();
    let Some(index) = parent.arguments.iter().position(|argument| argument.as_expression().is_some_and(|argument| strip_parens(argument).span() == span))
    else {
        return Vec::new();
    };
    let mut steps = Vec::new();
    let mut receiver = call_pipe_receiver(parent);

    while let Some(current) = receiver {
        steps.push(current);

        if let Expression::CallExpression(inner) = current {
            steps.extend(inner.arguments.iter().filter_map(Argument::as_expression));
        }

        receiver = pipe_receiver(current);
    }

    steps.extend(parent.arguments.iter().take(index).filter_map(Argument::as_expression));
    steps
}

/// The binding `identifier` names, looked up by name from its scope, as
/// upstream's `resolveIdentifier`.
pub fn resolve(semantic: &Semantic<'_>, identifier: &IdentifierReference<'_>) -> Option<SymbolId> {
    let scope = semantic.nodes().get_node(identifier.node_id()).scope_id();

    semantic.scoping().find_binding(scope, identifier.name)
}

/// The initializer of the single declarator `identifier` resolves to, or the
/// declared function.
pub fn resolve_initializer<'a>(semantic: &Semantic<'a>, identifier: &IdentifierReference<'_>) -> Option<Value<'a>> {
    let symbol = resolve(semantic, identifier)?;
    let scoping = semantic.scoping();

    if !scoping.symbol_redeclarations(symbol).is_empty() {
        return None;
    }

    match semantic.nodes().kind(scoping.symbol_declaration(symbol)) {
        AstKind::Function(function) if function.r#type == FunctionType::FunctionDeclaration => Some(Value::Function(function)),
        AstKind::VariableDeclarator(declarator) => declarator.init.as_ref().map(Value::Expression),
        _ => None,
    }
}

/// The generator passed last to `Effect.gen(...)`.
pub fn gen_function<'a>(call: &'a CallExpression<'a>, modules: &Modules) -> Option<&'a Function<'a>> {
    if callee_name(call, modules, Module::Effect) != Some("gen") {
        return None;
    }

    match call.arguments.last().and_then(Argument::as_expression).map(strip_parens) {
        Some(Expression::FunctionExpression(function)) => Some(function),
        _ => None,
    }
}

/// The generator that builds a service from a make: `Effect.gen(function* () {...})`,
/// optionally followed by `.pipe(...)`, or a function that returns one.
pub fn make_generator<'a>(nodes: &AstNodes<'a>, make: Value<'a>, modules: &Modules) -> Option<&'a Function<'a>> {
    let mut expression = match make {
        Value::Function(function) => unwrap(returned_expressions(nodes, FunctionNode::Function(function)).first()?),
        Value::Expression(expression) => {
            let expression = unwrap(expression);

            match FunctionNode::of(expression) {
                Some(function) => unwrap(returned_expressions(nodes, function).first()?),
                None => expression,
            }
        }
    };

    while let Some(receiver) = pipe_receiver(expression) {
        expression = receiver;
    }

    match expression {
        Expression::CallExpression(call) => gen_function(call, modules),
        _ => None,
    }
}
