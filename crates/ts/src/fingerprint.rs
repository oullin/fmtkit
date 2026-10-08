//! A structural fingerprint of a parsed program, used to check that every
//! pipeline step preserved the program it was given.
//!
//! Each node hashes its type, its payload (names, literal values, operators,
//! and modifier flags), and its children's hashes, bottom up. Spans,
//! whitespace, and quote style do not take part. Comments enter as a multiset
//! of their words. Two texts with equal fingerprints are, up to hash
//! collisions and the normalizations below, the same program.
//!
//! The normalizations are exactly what the steps may change:
//!
//! - Parentheses (`ParenthesizedExpression`, `TSParenthesizedType`), empty
//!   statements, and single-member union and intersection types are
//!   transparent: the formatter adds and drops them.
//! - A chain of one logical operator (`&&`, `||`, or `??`) hashes as one node
//!   over its operands in order, however it is grouped: the formatter drops
//!   the parentheses of `a && (b && c)`. Regrouping such a chain changes
//!   neither the order of evaluation nor where it short-circuits. Arithmetic
//!   and mixed chains keep their shape.
//! - A string-literal key and an identifier key hash alike, so the formatter
//!   may quote or unquote keys.
//! - A loop, `with`, or `if` body that is not a block hashes as a block of
//!   that one statement, so body wrapping is invisible. An `else if` stays.
//! - JSX text hashes as its words, and a whitespace-only `{" "}` child is
//!   dropped, so the formatter may re-flow JSX text.
//! - In a statement list, each maximal run of imports and each maximal run of
//!   `const` declarations hashes as a multiset, so the declaration reorder is
//!   invisible. Everything else in the list keeps its order.
//! - A class body hashes its properties, then its constructors, then its
//!   methods, each kind in source order, which is exactly the order the class
//!   reorder produces with its stable sort.
//! - A template element's raw text hashes with `\r\n` and `\r` read as `\n`,
//!   as ECMAScript reads them in both the raw and the cooked value.
//! - A template the formatter may hand to an [embedded-language
//!   formatter](crate::embed) hashes its quasis without their text: the
//!   template, its quasi count, and its expressions still take part.
//!
//! What the check gives up: the relative order of imports within an import
//! run, of consts within a const run, and of class members of different kinds
//! is not verified. Those are the orders the reorder steps own. Nor is the
//! text of an embedded-language template: CSS, GraphQL, HTML, and Markdown
//! formatting rewrites more than whitespace (`;` and quotes added, list
//! markers and hex colours normalised), and any normalisation loose enough to
//! accept that would also accept real damage while rejecting valid output.
//! The sub-formatters are oxfmt's own, which ran unchecked in v1.

use std::hash::Hasher;

use oxc_ast::AstKind;
use oxc_ast::ast::{
    Argument, ArrayExpressionElement, Comment, Expression, JSXAttributeName, JSXAttributeValue, JSXChild, JSXElement, JSXElementName, JSXExpression,
    LogicalOperator, ObjectPropertyKind, Program, PropertyKey, Statement, VariableDeclarationKind,
};
use oxc_ast_visit::Visit;
use oxc_span::{GetSpan, Span};
use rustc_hash::{FxHashSet, FxHasher};

use crate::passes::spacing::{MemberKind, classify_kind};

/// The fingerprint of `program`, parsed from `text`.
pub(crate) fn fingerprint(text: &str, program: &Program<'_>) -> u64 {
    let mut walker = Walker { frames: vec![Frame::root()], stack: Vec::with_capacity(256), text, comments: &program.comments, embedded: FxHashSet::default() };

    walker.visit_program(program);

    let mut comments: Vec<u64> = program.comments.iter().map(|comment| words(&text[comment.span.start as usize..comment.span.end as usize])).collect();

    comments.sort_unstable();

    let mut hasher = FxHasher::default();

    for (_, child) in &walker.stack {
        hasher.write_u64(*child);
    }

    for comment in comments {
        hasher.write_u64(comment);
    }

    hasher.finish()
}

/// Type tags beyond `AstType`'s `u8` range.
const NAME: u64 = 0x100;
const BLOCK: u64 = 0x101;
const WORD: u64 = 0x102;

/// A list entry's role in its list: imports and consts form reorderable runs,
/// class members sort by kind.
const OTHER: u8 = 0;
const IMPORT: u8 = 1;
const CONST: u8 = 2;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Children in source order.
    Ordered,
    /// A statement list: runs of imports and of consts are multisets.
    List,
    /// A class body: children grouped by member kind, in order within a kind.
    Members,
    /// The node vanishes; its children belong to its parent.
    Transparent,
    /// The node and its children vanish.
    Skip,
}

struct Frame {
    /// Where the node's children start on the shared child stack.
    start: usize,
    mode: Mode,
    /// The node's role in its parent list.
    class: u8,
    /// Leading children kept in order before a list (a `case` test).
    prefix: usize,
    /// Non-block bodies that hash as blocks.
    bodies: [Option<Span>; 2],
    /// The node is a non-block body; leaving it closes its synthetic block.
    wrapped: bool,
    attribute: bool,
    /// The node is a template an embedded formatter may rewrite.
    embedded: bool,
    /// The operator of the logical chain the node's children continue: set on
    /// a logical expression, and passed through parentheses.
    logical: Option<LogicalOperator>,
}

impl Frame {
    const fn root() -> Self {
        Self { start: 0, mode: Mode::Ordered, class: OTHER, prefix: 0, bodies: [None; 2], wrapped: false, attribute: false, embedded: false, logical: None }
    }
}

struct Walker<'t> {
    frames: Vec<Frame>,
    /// Finished child hashes with their list roles, shared by all frames.
    stack: Vec<(u8, u64)>,
    text: &'t str,
    comments: &'t [Comment],
    /// Start offsets of the embedded-language templates met so far; a site is
    /// recorded when its parent node is entered, before the template itself.
    embedded: FxHashSet<u32>,
}

impl Walker<'_> {
    fn parent(&self) -> &Frame {
        self.frames.last().expect("the root frame is never popped")
    }

    /// Hash the children above `frame.start` into one node and replace them with it.
    fn close(&mut self, frame: &Frame, tag: u64, kind: Option<AstKind<'_>>) {
        let children = &mut self.stack[frame.start..];
        let prefix = frame.prefix.min(children.len());

        match frame.mode {
            Mode::List => sort_runs(&mut children[prefix..]),
            Mode::Members => children.sort_by_key(|(class, _)| *class),
            _ => {}
        }

        let mut hasher = FxHasher::default();

        hasher.write_u64(tag);

        if let Some(kind) = kind {
            payload(kind, &mut hasher);
        }

        hasher.write_usize(children.len());

        for (_, child) in children.iter() {
            hasher.write_u64(*child);
        }

        self.stack.truncate(frame.start);
        self.stack.push((frame.class, hasher.finish()));
    }
}

impl<'a> Visit<'a> for Walker<'_> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        let span = kind.span();

        mark_embedded(kind, &mut self.embedded);

        let embedded =
            matches!(kind, AstKind::TemplateLiteral(_)) && (self.embedded.contains(&span.start) || has_language_comment(self.text, self.comments, span.start));
        let parent = self.frames.last_mut().expect("the root frame is never popped");
        let wrapped = parent.bodies.iter_mut().find(|body| **body == Some(span)).map(Option::take).is_some();

        if wrapped {
            let start = self.stack.len();

            self.frames.push(Frame { start, mode: Mode::List, ..Frame::root() });
        }

        let parent = self.parent();
        let logical = match kind {
            AstKind::LogicalExpression(it) => Some(it.operator),
            AstKind::ParenthesizedExpression(_) => parent.logical,
            _ => None,
        };
        let mode = match kind {
            AstKind::LogicalExpression(it) if parent.logical == Some(it.operator) => Mode::Transparent,
            _ => mode(kind, parent.attribute),
        };

        let class = match parent.mode {
            Mode::List => class(kind),
            Mode::Members => member_class(kind),
            _ => OTHER,
        };

        let prefix = match kind {
            AstKind::SwitchCase(case) => usize::from(case.test.is_some()),
            _ => 0,
        };

        let frame = Frame {
            start: self.stack.len(),
            mode,
            class,
            prefix,
            bodies: bodies(kind),
            wrapped,
            attribute: matches!(kind, AstKind::JSXAttribute(_)),
            embedded,
            logical,
        };

        self.frames.push(frame);

        if let AstKind::JSXText(text) = kind {
            self.stack.extend(text.value.as_str().split_whitespace().map(|word| (OTHER, hash_str(WORD, word))));
        }
    }

    fn leave_node(&mut self, kind: AstKind<'a>) {
        let frame = self.frames.pop().expect("enter and leave pair up");
        let embedded_text = matches!(kind, AstKind::TemplateElement(_)) && self.parent().embedded;

        match frame.mode {
            Mode::Transparent => {}
            Mode::Skip => self.stack.truncate(frame.start),
            _ => self.close(&frame, tag(kind), (!embedded_text).then_some(kind)),
        }

        if frame.wrapped {
            let block = self.frames.pop().expect("a wrapped body sits in its synthetic block");

            self.close(&block, BLOCK, None);
        }
    }
}

/// Tags whose templates oxc_formatter formats as another language.
const EMBEDDED_TAGS: [&str; 7] = ["css", "styled", "gql", "graphql", "html", "md", "markdown"];

/// Record the templates under `kind` that oxc_formatter may hand to an
/// embedded formatter (`print/template/embed/mod.rs`): a tagged template, a
/// `graphql(…)` argument, a JSX `css` prop, a `<style jsx>` child, and an
/// Angular `@Component` `template` or `styles`. Where oxc_formatter checks
/// more (a `md` template without `${}`), this records the superset, which
/// only skips checking text that stays as written anyway.
fn mark_embedded(kind: AstKind<'_>, out: &mut FxHashSet<u32>) {
    match kind {
        AstKind::TaggedTemplateExpression(it) if tag_name(&it.tag).is_some_and(|name| EMBEDDED_TAGS.contains(&name)) => {
            out.insert(it.quasi.span.start);
        }
        AstKind::CallExpression(it) if matches!(&it.callee, Expression::Identifier(ident) if ident.name == "graphql") => {
            out.extend(it.arguments.iter().filter_map(|argument| match argument {
                Argument::TemplateLiteral(template) => Some(template.span.start),
                _ => None,
            }));
        }
        AstKind::JSXAttribute(it) if matches!(&it.name, JSXAttributeName::Identifier(name) if name.name == "css") => {
            if let Some(JSXAttributeValue::ExpressionContainer(container)) = &it.value
                && let JSXExpression::TemplateLiteral(template) = &container.expression
            {
                out.insert(template.span.start);
            }
        }
        AstKind::JSXElement(it) if is_style_jsx(it) => {
            out.extend(it.children.iter().filter_map(|child| match child {
                JSXChild::ExpressionContainer(container) => match &container.expression {
                    JSXExpression::TemplateLiteral(template) => Some(template.span.start),
                    _ => None,
                },
                _ => None,
            }));
        }
        AstKind::Decorator(it) => {
            let Expression::CallExpression(call) = &it.expression else { return };

            if !matches!(&call.callee, Expression::Identifier(ident) if ident.name == "Component") {
                return;
            }

            let properties = call.arguments.iter().filter_map(|argument| match argument {
                Argument::ObjectExpression(object) => Some(object.properties.iter()),
                _ => None,
            });

            for property in properties.flatten() {
                let ObjectPropertyKind::ObjectProperty(property) = property else { continue };

                if property.computed || !matches!(&property.key, PropertyKey::StaticIdentifier(key) if key.name == "template" || key.name == "styles") {
                    continue;
                }

                match &property.value {
                    Expression::TemplateLiteral(template) => {
                        out.insert(template.span.start);
                    }
                    Expression::ArrayExpression(array) => out.extend(array.elements.iter().filter_map(|element| match element {
                        ArrayExpressionElement::TemplateLiteral(template) => Some(template.span.start),
                        _ => None,
                    })),
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

/// oxc_formatter's `get_tag_name`: `styled.div`, `styled(Button).attrs()`,
/// and `css.global` are tagged by their root identifier.
fn tag_name<'a>(expression: &'a Expression<'a>) -> Option<&'a str> {
    match expression.get_inner_expression() {
        Expression::Identifier(ident) => Some(ident.name.as_str()),
        Expression::StaticMemberExpression(member) => tag_name(&member.object),
        Expression::ComputedMemberExpression(member) => tag_name(&member.object),
        Expression::CallExpression(call) => tag_name(&call.callee),
        _ => None,
    }
}

fn is_style_jsx(element: &JSXElement<'_>) -> bool {
    let opening = &element.opening_element;

    matches!(&opening.name, JSXElementName::Identifier(name) if name.name == "style")
        && opening.attributes.iter().any(
            |attribute| matches!(attribute.as_attribute().map(|attribute| &attribute.name), Some(JSXAttributeName::Identifier(name)) if name.name == "jsx"),
        )
}

/// Whether the comment closest before `start` is `/* HTML */` or
/// `/* GraphQL */` with only whitespace after it, which oxc_formatter takes as
/// the language of the template at `start`.
fn has_language_comment(text: &str, comments: &[Comment], start: u32) -> bool {
    let before = comments.partition_point(|comment| comment.span.end <= start);
    let Some(comment) = before.checked_sub(1).and_then(|index| comments.get(index)) else {
        return false;
    };
    let content = comment.content_span();

    comment.is_block()
        && matches!(text.get(content.start as usize..content.end as usize), Some(" HTML " | " GraphQL "))
        && text.get(comment.span.end as usize..start as usize).is_some_and(|gap| gap.bytes().all(|byte| byte.is_ascii_whitespace()))
}

fn hash_str(tag: u64, text: &str) -> u64 {
    let mut hasher = FxHasher::default();

    hasher.write_u64(tag);
    hasher.write(text.as_bytes());

    hasher.finish()
}

fn words(text: &str) -> u64 {
    let mut hasher = FxHasher::default();

    for word in text.split_whitespace() {
        hasher.write(word.as_bytes());
        hasher.write_u8(0);
    }

    hasher.finish()
}

/// Sort each maximal run of imports and of consts by hash.
fn sort_runs(children: &mut [(u8, u64)]) {
    let mut index = 0;

    while index < children.len() {
        let class = children[index].0;
        let run = children[index..].iter().take_while(|(other, _)| *other == class).count();

        if class != OTHER {
            children[index..index + run].sort_unstable_by_key(|(_, hash)| *hash);
        }

        index += run;
    }
}

fn tag(kind: AstKind<'_>) -> u64 {
    match kind {
        AstKind::StringLiteral(_) | AstKind::IdentifierName(_) => NAME,
        AstKind::BlockStatement(_) => BLOCK,
        _ => u64::from(kind.ty() as u8),
    }
}

fn mode(kind: AstKind<'_>, in_attribute: bool) -> Mode {
    match kind {
        AstKind::ParenthesizedExpression(_) | AstKind::TSParenthesizedType(_) | AstKind::EmptyStatement(_) | AstKind::JSXText(_) => Mode::Transparent,
        AstKind::TSUnionType(union) if union.types.len() == 1 => Mode::Transparent,
        AstKind::TSIntersectionType(intersection) if intersection.types.len() == 1 => Mode::Transparent,
        AstKind::JSXExpressionContainer(container) => match &container.expression {
            JSXExpression::StringLiteral(literal) if !in_attribute && literal.value.as_str().trim().is_empty() => Mode::Skip,
            JSXExpression::StringLiteral(_) => Mode::Transparent,
            _ => Mode::Ordered,
        },
        AstKind::Program(_) | AstKind::BlockStatement(_) | AstKind::FunctionBody(_) | AstKind::StaticBlock(_) | AstKind::SwitchCase(_) => Mode::List,
        AstKind::ClassBody(_) => Mode::Members,
        _ => Mode::Ordered,
    }
}

fn class(kind: AstKind<'_>) -> u8 {
    match kind {
        AstKind::ImportDeclaration(_) => IMPORT,
        AstKind::VariableDeclaration(declaration) if declaration.kind == VariableDeclarationKind::Const => CONST,
        _ => OTHER,
    }
}

fn member_class(kind: AstKind<'_>) -> u8 {
    match classify_kind(kind) {
        Some(MemberKind::Property) => 1,
        Some(MemberKind::Constructor) => 2,
        Some(MemberKind::Method) => 3,
        None => OTHER,
    }
}

/// The non-block bodies of a control statement; an `else if` is not one.
fn bodies(kind: AstKind<'_>) -> [Option<Span>; 2] {
    let body = |statement: &Statement<'_>| (!matches!(statement, Statement::BlockStatement(_))).then(|| statement.span());

    match kind {
        AstKind::IfStatement(statement) => {
            let alternate = statement.alternate.as_ref().filter(|alternate| !matches!(alternate, Statement::IfStatement(_)));

            [body(&statement.consequent), alternate.and_then(body)]
        }
        AstKind::ForStatement(statement) => [body(&statement.body), None],
        AstKind::ForInStatement(statement) => [body(&statement.body), None],
        AstKind::ForOfStatement(statement) => [body(&statement.body), None],
        AstKind::WhileStatement(statement) => [body(&statement.body), None],
        AstKind::DoWhileStatement(statement) => [body(&statement.body), None],
        AstKind::WithStatement(statement) => [body(&statement.body), None],
        _ => [None; 2],
    }
}

/// The parts of a node that are not children: names, values, operators, flags.
fn payload(kind: AstKind<'_>, hasher: &mut FxHasher) {
    match kind {
        AstKind::IdentifierName(it) => hasher.write(it.name.as_str().as_bytes()),
        AstKind::IdentifierReference(it) => hasher.write(it.name.as_str().as_bytes()),
        AstKind::BindingIdentifier(it) => hasher.write(it.name.as_str().as_bytes()),
        AstKind::LabelIdentifier(it) => hasher.write(it.name.as_str().as_bytes()),
        AstKind::PrivateIdentifier(it) => hasher.write(it.name.as_str().as_bytes()),
        AstKind::JSXIdentifier(it) => hasher.write(it.name.as_str().as_bytes()),
        AstKind::StringLiteral(it) => hasher.write(it.value.as_str().as_bytes()),
        AstKind::TemplateElement(it) => {
            // ECMAScript's raw value reads `\r\n` and `\r` as `\n`; oxc keeps the source text.
            let raw = it.value.raw.as_str();

            if raw.contains('\r') {
                hasher.write(raw.replace("\r\n", "\n").replace('\r', "\n").as_bytes());
            } else {
                hasher.write(raw.as_bytes());
            }
        }
        AstKind::BigIntLiteral(it) => hasher.write(it.value.as_str().as_bytes()),
        AstKind::Hashbang(it) => hasher.write(it.value.as_str().as_bytes()),
        AstKind::NumericLiteral(it) => hasher.write_u64(it.value.to_bits()),
        AstKind::BooleanLiteral(it) => hasher.write_u8(u8::from(it.value)),
        AstKind::RegExpLiteral(it) => {
            hasher.write(it.regex.pattern.text.as_str().as_bytes());
            hasher.write_u8(it.regex.flags.bits());
        }
        AstKind::BinaryExpression(it) => hasher.write_u8(it.operator as u8),
        AstKind::LogicalExpression(it) => hasher.write_u8(it.operator as u8),
        AstKind::UnaryExpression(it) => hasher.write_u8(it.operator as u8),
        AstKind::AssignmentExpression(it) => hasher.write_u8(it.operator as u8),
        AstKind::UpdateExpression(it) => {
            hasher.write_u8(it.operator as u8);
            hasher.write_u8(u8::from(it.prefix));
        }
        AstKind::TSTypeOperator(it) => hasher.write_u8(it.operator as u8),
        AstKind::VariableDeclaration(it) => hasher.write_u8(it.kind as u8),
        AstKind::Function(it) => hasher.write_u8(u8::from(it.r#async) | u8::from(it.generator) << 1),
        AstKind::ArrowFunctionExpression(it) => hasher.write_u8(u8::from(it.r#async)),
        AstKind::ForOfStatement(it) => hasher.write_u8(u8::from(it.r#await)),
        AstKind::ObjectProperty(it) => {
            hasher.write_u8(it.kind as u8);
            hasher.write_u8(u8::from(it.method) | u8::from(it.shorthand) << 1 | u8::from(it.computed) << 2);
        }
        AstKind::MethodDefinition(it) => {
            hasher.write_u8(it.kind as u8);
            hasher.write_u8(u8::from(it.r#static) | u8::from(it.computed) << 1);
        }
        AstKind::PropertyDefinition(it) => hasher.write_u8(u8::from(it.r#static) | u8::from(it.computed) << 1),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use oxc_allocator::Allocator;
    use oxc_span::SourceType;

    use super::fingerprint;
    use crate::syntax::parse;

    fn print(text: &str, source_type: SourceType) -> u64 {
        let allocator = Allocator::default();
        let program = parse(&allocator, text, source_type).expect("parses");

        fingerprint(text, program)
    }

    fn same(a: &str, b: &str) -> bool {
        print(a, SourceType::tsx()) == print(b, SourceType::tsx())
    }

    #[test]
    fn ignores_layout_quotes_parentheses_and_key_quoting() {
        assert!(same("const a = {'b': \"c\"};", "const a = {\n\tb: 'c',\n};\n"));
        assert!(same("const a = (1 + 2) * (b);", "const a = (1 + 2) * b"));
        assert!(same("type A = (string | number)[];;", "type A = (string | number)[]"));
        assert!(same("const r = /a/gi;", "const r = /a/ig;"));
    }

    #[test]
    fn treats_wrapped_bodies_as_blocks() {
        assert!(same("if (a) b(); else if (c) d(); else e();", "if (a) {\n\tb();\n} else if (c) {\n\td();\n} else {\n\te();\n}"));
        assert!(same("for (;;) x++;", "for (;;) {\n\tx++;\n}"));
        assert!(!same("if (a) b(); else if (c) d();", "if (a) b(); else {\n\tif (c) d();\n}"));
    }

    #[test]
    fn tolerates_reordered_import_and_const_runs_only() {
        assert!(same(
            "import a from 'a';\nimport b from 'b';\nconst x = 1;\nconst y = 2;\nrun();",
            "import b from 'b';\nimport a from 'a';\nconst y = 2;\nconst x = 1;\nrun();"
        ));
        assert!(!same("const x = 1;\nrun();\nconst y = 2;", "const y = 2;\nrun();\nconst x = 1;"));
        assert!(!same("let x = 1;\nlet y = 2;", "let y = 2;\nlet x = 1;"));
    }

    #[test]
    fn tolerates_class_members_sorted_by_kind_only() {
        assert!(same("class A {\n\trun() {}\n\tb = 1;\n\tconstructor() {}\n}", "class A {\n\tb = 1;\n\tconstructor() {}\n\trun() {}\n}"));
        assert!(!same("class A {\n\tone() {}\n\ttwo() {}\n}", "class A {\n\ttwo() {}\n\tone() {}\n}"));
    }

    #[test]
    fn ignores_jsx_text_reflow() {
        assert!(same("const v = <p>Hello <b>world</b> again</p>;", "const v = (\n\t<p>\n\t\tHello{' '}\n\t\t<b>world</b> again\n\t</p>\n);"));
        assert!(same("const v = <a b={'c'} />;", "const v = <a b=\"c\" />;"));
        assert!(!same("const v = <p>Hello</p>;", "const v = <p>Goodbye</p>;"));
    }

    #[test]
    fn ignores_the_text_of_embedded_language_templates_only() {
        assert!(same("const s = css`a{color:red}`;", "const s = css`\n\ta {\n\t\tcolor: red;\n\t}\n`;"));
        assert!(same("const s = styled.div`color:${c}`;", "const s = styled.div`\n\tcolor: ${c};\n`;"));
        assert!(same("const q = graphql(`{a}`);", "const q = graphql(`\n\t{\n\t\ta\n\t}\n`);"));
        assert!(same("const h = /* HTML */ `<p>a</p>`;", "const h = /* HTML */ `\n\t<p>a</p>\n`;"));
        assert!(same("const v = <div css={`a:b`} />;", "const v = <div css={`\n\ta: b;\n`} />;"));
        assert!(same("@Component({ styles: [`a{b:c}`] })\nclass A {}", "@Component({ styles: [`\n\ta {\n\t\tb: c;\n\t}\n`] })\nclass A {}"));
        assert!(!same("const s = css`a{color:${c}}`;", "const s = css`a{color:${d}}`;"));
        assert!(!same("const s = css`a${b}`;", "const s = css`a`;"));
        assert!(!same("const s = `a{color:red}`;", "const s = `a { color: red; }`;"));
        assert!(!same("const s = sql`select  1`;", "const s = sql`select 1`;"));
        assert!(!same("const h = /* note */ `<p>a</p>`;", "const h = /* note */ `<p>b</p>`;"));
    }

    #[test]
    fn flattens_chains_of_one_logical_operator_only() {
        // Fuzz finding `oxfmt-drops-logical-parens`.
        assert!(same("{e&&(b&&c)}", "{\n\te && b && c;\n}"));
        assert!(same("a || (b || (c || d));", "(a || b) || (c || d);"));
        assert!(same("a ?? (b ?? c);", "a ?? b ?? c;"));
        assert!(same("f(a && ((b && c)));", "f((a && b) && c);"));
        assert!(!same("a && (b || c);", "a && b || c;"));
        assert!(!same("a || (b && c) || d;", "a || b && c || d || e;"));
        assert!(!same("a && (b && c);", "a && (c && b);"));
        assert!(!same("a && b && c;", "a || b || c;"));
        assert!(!same("a - (b - c);", "a - b - c;"));
        assert!(!same("a + (b + c);", "a + b + c;"));
        assert!(!same("a ** (b ** c);", "(a ** b) ** c;"));
    }

    #[test]
    fn reads_carriage_returns_in_templates_as_line_feeds() {
        assert!(same("a`x\r\\1`;", "a`x\n\\1`;"));
        assert!(same("`x\r\ny${a}\r`;", "`x\ny${a}\n`;"));
        assert!(!same("`x\r`;", "`x`;"));
        assert!(!same("'x\\r';", "'x\\n';"));
    }

    #[test]
    fn keeps_an_await_apart_from_a_call_of_await() {
        // Fuzz finding `oxfmt-await-numeric-member`: oxfmt prints `await 1e3.a`
        // as `await (1e3).a`. In a `.tsx` file, whose module kind is
        // unambiguous, that calls a function named `await`; in a module it is
        // the same program.
        let tsx = crate::syntax::source_type(fmtkit_core::Lang::Tsx, "a.tsx").expect("a script");
        let mts = crate::syntax::source_type(fmtkit_core::Lang::Mts, "a.mts").expect("a script");

        assert_ne!(print("await 1e3.a;", tsx), print("await (1e3).a;", tsx));
        assert_eq!(print("await 1e3.a;", mts), print("await (1e3).a;", mts));
    }

    #[test]
    fn detects_lost_code_and_comments() {
        assert!(!same("a();\nb();", "a();"));
        assert!(!same("a = b + c;", "a = b - c;"));
        assert!(!same("const s = 'x  y';", "const s = 'x y';"));
        assert!(!same("a(); // note", "a();"));
        assert!(same("/**\n * Doc.\n */\nfunction f() {}", "/**\n     * Doc.\n     */\nfunction f() {}"));
        assert!(!same("async function f() {}", "function f() {}"));
    }
}
