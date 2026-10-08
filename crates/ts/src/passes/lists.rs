//! The sibling lists the blank-line and declaration passes walk: statement
//! lists, class bodies, and switch cases, in ESTree terms.

use oxc_ast::AstKind;
use oxc_ast::ast::{ClassElement, Directive, Program, Statement, SwitchCase, VariableDeclarationKind};
use oxc_ast_visit::Visit;
use oxc_span::{GetSpan, Span};

/// One entry of a sibling list. A directive is an ESTree `ExpressionStatement`.
#[derive(Clone, Copy)]
pub(crate) enum Item<'a> {
    Directive(&'a Directive<'a>),
    Statement(&'a Statement<'a>),
    Member(&'a ClassElement<'a>),
    Case(&'a SwitchCase<'a>),
}

impl<'a> Item<'a> {
    pub(crate) fn span(self) -> Span {
        match self {
            Self::Directive(directive) => directive.span,
            Self::Statement(statement) => statement.span(),
            Self::Member(member) => member.span(),
            Self::Case(case) => case.span,
        }
    }

    pub(crate) fn statement(self) -> Option<&'a Statement<'a>> {
        match self {
            Self::Statement(statement) => Some(statement),
            _ => None,
        }
    }

    pub(crate) fn is_import(self) -> bool {
        matches!(self.statement(), Some(Statement::ImportDeclaration(_)))
    }

    pub(crate) fn declaration_kind(self) -> Option<VariableDeclarationKind> {
        match self.statement() {
            Some(Statement::VariableDeclaration(declaration)) => Some(declaration.kind),
            _ => None,
        }
    }

    pub(crate) fn is_const(self) -> bool {
        self.declaration_kind() == Some(VariableDeclarationKind::Const)
    }
}

/// Call `visit` with every sibling list below `program`, outermost first.
pub(crate) fn for_each_list<'a>(program: &'a Program<'a>, visit: impl FnMut(&[Item<'a>])) {
    let mut lists = Lists { visit, items: Vec::new() };

    lists.visit_program(program);
}

struct Lists<'a, F> {
    visit: F,
    items: Vec<Item<'a>>,
}

impl<'a, F: FnMut(&[Item<'a>])> Lists<'a, F> {
    fn emit(&mut self, items: impl IntoIterator<Item = Item<'a>>) {
        self.items.clear();
        self.items.extend(items);
        (self.visit)(&self.items);
    }
}

impl<'a, F: FnMut(&[Item<'a>])> Visit<'a> for Lists<'a, F> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        match kind {
            AstKind::Program(program) => self.emit(program.directives.iter().map(Item::Directive).chain(program.body.iter().map(Item::Statement))),
            AstKind::BlockStatement(block) => self.emit(block.body.iter().map(Item::Statement)),
            AstKind::StaticBlock(block) => self.emit(block.body.iter().map(Item::Statement)),
            AstKind::SwitchCase(case) => self.emit(case.consequent.iter().map(Item::Statement)),
            AstKind::SwitchStatement(switch) => self.emit(switch.cases.iter().map(Item::Case)),
            AstKind::ClassBody(body) => self.emit(body.body.iter().map(Item::Member)),
            // An expression-bodied arrow has no `FunctionBody`, as in ESTree.
            AstKind::FunctionBody(body) => self.emit(body.directives.iter().map(Item::Directive).chain(body.statements.iter().map(Item::Statement))),
            _ => {}
        }
    }
}
