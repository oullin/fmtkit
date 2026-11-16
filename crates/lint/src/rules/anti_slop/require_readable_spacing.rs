//! `anti-slop/require-readable-spacing`: a blank line between the statements
//! the anti-slop spacing policy keeps apart, fixed by inserting one.
//!
//! A port of the vendored ESLint Stylistic `padding-line-between-statements`
//! under the one fixed configuration upstream gives it, rather than the whole
//! configurable rule. Consecutive statements of one statement list (a program,
//! block, function body, static block, namespace body, switch case, or the
//! cases of a switch) need a blank line between them when:
//!
//! - the first is an import and the second anything but an import;
//! - either is a top-level statement other than an import (a labeled
//!   statement counts by its body, which is not top level);
//! - either is a function, class, interface or type alias declaration
//!   (keyword-led: `export`, `declare`, `abstract` or a decorator first does
//!   not count, as the upstream keyword test reads the first token);
//! - either is a `const`, `let`, `var` or `using` declaration spanning lines;
//! - the second is a `return`, `if`, `switch`, `try`, `for`, `while` or `do`;
//! - the first is block-like: it ends in the brace of a block or switch, or
//!   is a do-while with a block body, or an IIFE.
//!
//! Deliberately skipped, as upstream: two imports in a row, and overload
//! signatures followed by another signature or the implementation. Existing
//! blank lines are never removed. A blank line counts only between tokens, so
//! one inside a comment does not; comments and semicolons on the line of the
//! first statement stay with it, and the fix inserts the blank line after them.
//! `export =` is not a statement upstream visits, so the statements around it
//! pair with each other. Interface and type literal members never need one
//! under this policy and are not visited. The rule takes no options.
//!
//! Not ported: a multi-line template literal inside a skipped `export =`
//! counts its own blank lines, where upstream reads it as one token.

// Derived from ESLint Stylistic's `padding-line-between-statements`
// (https://github.com/eslint-stylistic/eslint-stylistic), itself derived from
// ESLint's rule of the same name.
//
// MIT License
//
// Copyright OpenJS Foundation and other contributors, <www.openjsf.org>
// Copyright (c) 2023-PRESENT ESLint Stylistic contributors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use oxc_ast::AstKind;
use oxc_ast::ast::{ChainElement, Comment, Declaration, Directive, Expression, FunctionType, Statement, SwitchCase, VariableDeclarationKind};
use oxc_semantic::{AstNode, Semantic};
use oxc_span::{GetSpan, Span};
use rustc_hash::FxHashSet;
use serde_json::Value;

use fmtkit_core::Edit;

use super::super::{Context, Rule};
use super::shared::{is_js_space, strip_parens};

pub const NAME: &str = "anti-slop/require-readable-spacing";

const MESSAGE: &str = "Expected blank line before this statement.";

/// Upstream's schema is empty: the policy is fixed.
pub fn build(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    if options.is_empty() { Ok(Box::new(RequireReadableSpacing)) } else { Err("takes no options".to_owned()) }
}

struct RequireReadableSpacing;

impl Rule for RequireReadableSpacing {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        match node.kind() {
            AstKind::Program(program) => check(ctx, true, directives(&program.directives).chain(statements(&program.body))),
            AstKind::FunctionBody(body) => check(ctx, false, directives(&body.directives).chain(statements(&body.statements))),
            AstKind::TSModuleBlock(block) => check(ctx, false, directives(&block.directives).chain(statements(&block.body))),
            AstKind::BlockStatement(block) => check(ctx, false, statements(&block.body)),
            AstKind::StaticBlock(block) => check(ctx, false, statements(&block.body)),
            AstKind::SwitchCase(case) => check(ctx, false, statements(&case.consequent)),
            AstKind::SwitchStatement(switch) => check(ctx, false, switch.cases.iter().map(Item::Case)),
            _ => {}
        }
    }
}

/// One member of a statement list, as upstream's ESTree has it: directives are
/// expression statements, and switch cases pair with each other.
#[derive(Clone, Copy)]
enum Item<'a> {
    Directive(&'a Directive<'a>),
    Statement(&'a Statement<'a>),
    Case(&'a SwitchCase<'a>),
}

impl<'a> Item<'a> {
    fn span(self) -> Span {
        match self {
            Item::Directive(directive) => directive.span,
            Item::Statement(statement) => statement.span(),
            Item::Case(case) => case.span,
        }
    }

    /// The statement a label wraps, as upstream matches it.
    fn unlabeled(self) -> Self {
        let mut item = self;

        while let Item::Statement(Statement::LabeledStatement(labeled)) = item {
            item = Item::Statement(&labeled.body);
        }

        item
    }

    fn statement(self) -> Option<&'a Statement<'a>> {
        if let Item::Statement(statement) = self { Some(statement) } else { None }
    }
}

fn directives<'a>(directives: &'a [Directive<'a>]) -> impl Iterator<Item = Item<'a>> {
    directives.iter().map(Item::Directive)
}

/// The statements upstream visits: `export =` is neither a statement nor a
/// declaration to its selector.
fn statements<'a>(statements: &'a [Statement<'a>]) -> impl Iterator<Item = Item<'a>> {
    statements.iter().filter(|statement| !matches!(statement, Statement::TSExportAssignment(_))).map(Item::Statement)
}

fn check<'a>(ctx: &mut Context<'_, 'a>, top_level: bool, items: impl Iterator<Item = Item<'a>>) {
    let mut previous: Option<Item<'a>> = None;

    for item in items {
        if let Some(previous) = previous.replace(item) {
            verify(ctx, top_level, previous, item);
        }
    }
}

fn verify<'a>(ctx: &mut Context<'_, 'a>, top_level: bool, previous: Item<'a>, next: Item<'a>) {
    let semantic = ctx.semantic;
    let text = Text { source: semantic.source_text(), comments: semantic.comments() };
    let last = text.actual_last_token_end(previous.span());
    let next_span = next.span();

    if text.has_padding_line(last, next_span.start) || !needs_padding(ctx, &text, top_level, previous, next) {
        return;
    }

    let (at, insert) = text.insertion(last, next_span.start);

    ctx.report_with_fix(text.report_span(next_span), MESSAGE, vec![Edit::insert(at, insert)]);
}

/// Whether the policy wants a blank line between `previous` and `next`: the
/// last matching configuration wins, and only the two `any` ones follow the
/// `always` ones.
fn needs_padding(ctx: &mut Context<'_, '_>, text: &Text<'_>, top_level: bool, previous: Item<'_>, next: Item<'_>) -> bool {
    let (prev, next_inner) = (previous.unlabeled(), next.unlabeled());

    if (is_import(prev) && is_import(next_inner)) || (is_overload(prev) && is_function_or_overload(next_inner)) {
        return false;
    }

    is_import(prev)
        || is_top_level(top_level, previous)
        || is_top_level(top_level, next)
        || is_declaration(text, prev)
        || is_declaration(text, next_inner)
        || is_multiline_binding(text, prev)
        || is_multiline_binding(text, next_inner)
        || is_control(next_inner)
        || is_block_like(ctx, text, prev)
}

fn is_import(item: Item<'_>) -> bool {
    matches!(item.statement(), Some(Statement::ImportDeclaration(_)))
}

/// `Program > :not(ImportDeclaration)`, tested on the unlabeled statement,
/// whose parent is the label when there is one.
fn is_top_level(top_level: bool, item: Item<'_>) -> bool {
    top_level && !matches!(item.statement(), Some(Statement::LabeledStatement(_) | Statement::ImportDeclaration(_)))
}

/// A bodiless overload signature, exported or not.
fn is_overload(item: Item<'_>) -> bool {
    match item.statement() {
        Some(Statement::FunctionDeclaration(function)) => function.r#type == FunctionType::TSDeclareFunction,
        Some(Statement::ExportDeclaration(export)) => {
            matches!(&export.declaration, Declaration::FunctionDeclaration(function) if function.r#type == FunctionType::TSDeclareFunction)
        }
        _ => false,
    }
}

/// An overload signature or a function declaration, exported or not.
fn is_function_or_overload(item: Item<'_>) -> bool {
    match item.statement() {
        Some(Statement::FunctionDeclaration(_)) => true,
        Some(Statement::ExportDeclaration(export)) => matches!(export.declaration, Declaration::FunctionDeclaration(_)),
        _ => false,
    }
}

/// `function`, `class`, `interface` or `type`.
fn is_declaration(text: &Text<'_>, item: Item<'_>) -> bool {
    match item.statement() {
        Some(Statement::FunctionDeclaration(function)) => function.r#type == FunctionType::FunctionDeclaration,
        Some(Statement::ClassDeclaration(class)) => text.starts_with_keyword(class.span.start, "class"),
        Some(Statement::TSInterfaceDeclaration(interface)) => text.starts_with_keyword(interface.span.start, "interface"),
        Some(Statement::TSTypeAliasDeclaration(alias)) => text.starts_with_keyword(alias.span.start, "type"),
        _ => false,
    }
}

/// `multiline-const`, `multiline-let`, `multiline-var` or `multiline-using`.
fn is_multiline_binding(text: &Text<'_>, item: Item<'_>) -> bool {
    let Some(Statement::VariableDeclaration(declaration)) = item.statement() else {
        return false;
    };

    let keyword_led = match declaration.kind {
        VariableDeclarationKind::Var => text.starts_with_keyword(declaration.span.start, "var"),
        VariableDeclarationKind::Let => text.starts_with_keyword(declaration.span.start, "let"),
        VariableDeclarationKind::Const => text.starts_with_keyword(declaration.span.start, "const"),
        VariableDeclarationKind::Using | VariableDeclarationKind::AwaitUsing => true,
    };

    keyword_led && text.has_line_break(declaration.span.start, declaration.span.end)
}

/// `return`, `if`, `switch`, `try`, `for`, `while` or `do`.
fn is_control(item: Item<'_>) -> bool {
    matches!(
        item.statement(),
        Some(
            Statement::ReturnStatement(_)
                | Statement::IfStatement(_)
                | Statement::SwitchStatement(_)
                | Statement::TryStatement(_)
                | Statement::ForStatement(_)
                | Statement::ForInStatement(_)
                | Statement::ForOfStatement(_)
                | Statement::WhileStatement(_)
                | Statement::DoWhileStatement(_)
        )
    )
}

/// A do-while with a block body, an IIFE, or a statement whose last token
/// other than semicolons closes a block or a switch.
fn is_block_like(ctx: &mut Context<'_, '_>, text: &Text<'_>, item: Item<'_>) -> bool {
    match item.statement() {
        Some(Statement::DoWhileStatement(statement)) if matches!(statement.body, Statement::BlockStatement(_)) => return true,
        Some(Statement::ExpressionStatement(statement)) if is_iife(&statement.expression) => return true,
        _ => {}
    }

    let Some(end) = text.last_non_semicolon_token_end(item.span()) else {
        return false;
    };

    text.char_before(end) == Some('}') && ctx.shared(BlockEnds::new).0.contains(&end)
}

/// `(function () {})()`, `!function () {}()`, `(0, () => {})()`, optional calls included.
fn is_iife(expression: &Expression<'_>) -> bool {
    let mut expression = strip_parens(expression);

    if let Expression::UnaryExpression(unary) = expression {
        expression = strip_parens(&unary.argument);
    }

    let callee = match expression {
        Expression::CallExpression(call) => &call.callee,
        Expression::ChainExpression(chain) => match &chain.expression {
            ChainElement::CallExpression(call) => &call.callee,
            _ => return false,
        },
        _ => return false,
    };

    let mut callee = strip_parens(callee);

    while let Expression::SequenceExpression(sequence) = callee
        && let Some(last) = sequence.expressions.last()
    {
        callee = strip_parens(last);
    }

    matches!(callee, Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_))
}

/// Where the ESTree nodes that close with a block's brace end: blocks,
/// function bodies (ESTree's blocks) and switches. The node owning a closing
/// brace is the deepest one containing it, and no node inside a block ends at
/// its brace, so a brace closes a block exactly when a block ends there.
struct BlockEnds(FxHashSet<u32>);

impl BlockEnds {
    fn new(semantic: &Semantic<'_>) -> Self {
        let mut ends = FxHashSet::default();

        for node in semantic.nodes().iter() {
            match node.kind() {
                AstKind::BlockStatement(block) => {
                    ends.insert(block.span.end);
                }
                AstKind::FunctionBody(body) => {
                    ends.insert(body.span.end);
                }
                AstKind::SwitchStatement(switch) => {
                    ends.insert(switch.span.end);
                }
                _ => {}
            }
        }

        Self(ends)
    }
}

/// The source between tokens: whitespace, comments, and the odd semicolon.
struct Text<'s> {
    source: &'s str,
    comments: &'s [Comment],
}

impl Text<'_> {
    fn char_at(&self, position: u32) -> Option<char> {
        self.source.get(position as usize..)?.chars().next()
    }

    fn char_before(&self, position: u32) -> Option<char> {
        self.source.get(..position as usize)?.chars().next_back()
    }

    fn comment_starting_at(&self, position: u32) -> Option<&Comment> {
        let index = self.comments.binary_search_by_key(&position, |comment| comment.span.start).ok()?;

        self.comments.get(index)
    }

    fn comment_ending_at(&self, position: u32) -> Option<&Comment> {
        let index = self.comments.binary_search_by_key(&position, |comment| comment.span.end).ok()?;

        self.comments.get(index)
    }

    /// Whether a line terminator lies in `[start, end)`, comments included.
    fn has_line_break(&self, start: u32, end: u32) -> bool {
        self.source.get(start as usize..end as usize).is_some_and(|text| text.contains(is_line_break))
    }

    /// Whether the token at `start` is the keyword `keyword`.
    fn starts_with_keyword(&self, start: u32, keyword: &str) -> bool {
        self.source
            .get(start as usize..)
            .and_then(|rest| rest.strip_prefix(keyword))
            .is_some_and(|after| !after.starts_with(|c: char| c.is_alphanumeric() || matches!(c, '_' | '$' | '\\' | '\u{200c}' | '\u{200d}')))
    }

    /// The end of the token before `position`, skipping whitespace and comments.
    fn token_end_before(&self, mut position: u32) -> Option<u32> {
        loop {
            if let Some(comment) = self.comment_ending_at(position) {
                position = comment.span.start;
                continue;
            }

            let c = self.char_before(position)?;

            if !is_js_space(c) {
                return Some(position);
            }

            position -= u32::try_from(c.len_utf8()).ok()?;
        }
    }

    /// The start of the token at or after `position`, skipping whitespace and comments.
    fn token_start_after(&self, mut position: u32) -> Option<u32> {
        loop {
            if let Some(comment) = self.comment_starting_at(position) {
                position = comment.span.end;
                continue;
            }

            let c = self.char_at(position)?;

            if !is_js_space(c) {
                return Some(position);
            }

            position += u32::try_from(c.len_utf8()).ok()?;
        }
    }

    /// The first position at or after `position` that is not whitespace.
    fn skip_spaces(&self, mut position: u32) -> u32 {
        while let Some(c) = self.char_at(position)
            && is_js_space(c)
        {
            position += u32::try_from(c.len_utf8()).unwrap_or(1);
        }

        position
    }

    /// Where a statement's last token ends, ignoring the semicolon of
    /// semicolon-less style: `foo()\n;[1].forEach(bar)` ends at `)`.
    fn actual_last_token_end(&self, span: Span) -> u32 {
        if self.char_before(span.end) != Some(';') {
            return span.end;
        }

        let semicolon = span.end - 1;
        let Some(before) = self.token_end_before(semicolon).filter(|&before| before > span.start) else {
            return span.end;
        };
        let Some(after) = self.token_start_after(span.end) else {
            return span.end;
        };

        if self.has_line_break(before, semicolon) && !self.has_line_break(span.end, after) { before } else { span.end }
    }

    /// Where a statement's last token other than semicolons ends.
    fn last_non_semicolon_token_end(&self, span: Span) -> Option<u32> {
        let mut position = span.end;

        loop {
            position = self.token_end_before(position).filter(|&end| end > span.start)?;

            if self.char_before(position) != Some(';') {
                return Some(position);
            }

            position -= 1;
        }
    }

    /// Whether two tokens (comments included) between `start` and `end` have
    /// a whole line between them. Every character other than whitespace and
    /// comments counts as a token.
    fn has_padding_line(&self, start: u32, end: u32) -> bool {
        let first = self.comments.partition_point(|comment| comment.span.start < start);
        let mut comments = self.comments.get(first..).unwrap_or_default().iter().peekable();
        let mut breaks = 0;
        let mut position = start;

        while position < end {
            if let Some(comment) = comments.next_if(|comment| comment.span.start <= position) {
                breaks = 0;
                position = position.max(comment.span.end);
                continue;
            }

            let Some(c) = self.char_at(position) else { break };
            let mut width = c.len_utf8();

            if c == '\r' && self.char_at(position + 1) == Some('\n') {
                width = 2;
            }

            if is_line_break(c) {
                breaks += 1;

                if breaks >= 2 {
                    return true;
                }
            } else if !is_js_space(c) {
                breaks = 0;
            }

            position += u32::try_from(width).unwrap_or(1);
        }

        false
    }

    /// Where the fix inserts, and what: after the statement's last token and
    /// any tokens on its line, a line break, doubled when the next statement
    /// shares that line.
    fn insertion(&self, last: u32, next_start: u32) -> (u32, &'static str) {
        let mut position = last;

        loop {
            let token = self.skip_spaces(position);

            if token >= next_start {
                break;
            }

            if self.has_line_break(position, token) {
                return (position, "\n");
            }

            position = match self.comment_starting_at(token) {
                Some(comment) => comment.span.end,
                None => token + self.char_at(token).map_or(1, |c| u32::try_from(c.len_utf8()).unwrap_or(1)),
            };
        }

        (position, if self.has_line_break(position, next_start) { "\n" } else { "\n\n" })
    }

    /// The statement's span on its first line.
    fn report_span(&self, span: Span) -> Span {
        let first_line = self.source.get(span.start as usize..span.end as usize).and_then(|text| text.find(is_line_break));

        match first_line.and_then(|length| u32::try_from(length).ok()) {
            Some(length) => Span::new(span.start, span.start + length),
            None => span,
        }
    }
}

fn is_line_break(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}
