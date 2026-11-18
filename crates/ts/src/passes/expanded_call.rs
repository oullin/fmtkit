//! Put each argument of a plain call or a `new` expression on its own line
//! when one of them is a call, an object, an array, or a conditional with one
//! of those in a branch.

use fmtkit_core::{Edit, EditSet};
use oxc_ast::AstKind;
use oxc_ast::ast::{Argument, CallExpression, Comment, Expression, NewExpression, Program};
use oxc_ast_visit::Visit;
use oxc_span::{GetSpan, Span};

use crate::syntax::{Chained, argument_parens, call_parens, has_comment_between, indent_unit, line_indent, unwrap_expression};

/// One rewrite of the argument parentheses per expandable call; a call inside
/// another rewritten call is formatted by the outer rewrite.
pub(crate) fn edits<'a>(text: &'a str, program: &'a Program<'a>) -> EditSet {
    let mut collector = Collector { ancestors: Vec::new(), calls: Vec::new(), templates: Vec::new() };

    collector.visit_program(program);

    let writer = Writer { text, unit: indent_unit(text), comments: &program.comments, templates: &collector.templates };
    let mut edits = EditSet::new();

    for call in collector.calls {
        let Some((open, close)) = call.parens(text) else { continue };

        if has_comment_between(writer.comments, open, close) {
            continue;
        }

        let indent = line_indent(text, call.span().start);
        let Some(replacement) = writer.call_parens(call, indent) else { continue };

        if replacement != text[open as usize..=close as usize] {
            edits.push(Edit::new(open, close + 1, replacement));
        }
    }

    edits.retain_non_overlapping();

    edits
}

fn is_complex_argument(argument: &Argument<'_>) -> bool {
    argument.as_expression().is_some_and(is_complex)
}

/// A call, an object, an array, or a conditional with one of those in a branch.
fn is_complex(expression: &Expression<'_>) -> bool {
    match unwrap_expression(expression) {
        Chained::Call(_) | Chained::Other(Expression::ObjectExpression(_) | Expression::ArrayExpression(_)) => true,
        Chained::Other(Expression::ConditionalExpression(conditional)) => is_complex(&conditional.consequent) || is_complex(&conditional.alternate),
        _ => false,
    }
}

/// A call or `new` expression whose argument list may be expanded.
#[derive(Clone, Copy)]
enum Invocation<'a> {
    Call(&'a CallExpression<'a>),
    New(&'a NewExpression<'a>),
}

impl<'a> Invocation<'a> {
    fn of_kind(kind: AstKind<'a>) -> Option<Self> {
        match kind {
            AstKind::CallExpression(call) => Some(Self::Call(call)),
            AstKind::NewExpression(new) => Some(Self::New(new)),
            _ => None,
        }
    }

    fn of_argument(argument: &'a Argument<'a>) -> Option<Self> {
        match argument {
            Argument::CallExpression(call) => Some(Self::Call(call)),
            Argument::NewExpression(new) => Some(Self::New(new)),
            _ => None,
        }
    }

    fn span(self) -> Span {
        match self {
            Self::Call(call) => call.span,
            Self::New(new) => new.span,
        }
    }

    fn arguments(self) -> &'a [Argument<'a>] {
        match self {
            Self::Call(call) => &call.arguments,
            Self::New(new) => &new.arguments,
        }
    }

    /// Method calls are left to the fluent-chain passes; a constructor's
    /// callee never starts a chain.
    fn should_expand(self) -> bool {
        let method = matches!(self, Self::Call(call) if matches!(unwrap_expression(&call.callee), Chained::Member(_)));

        !method && self.arguments().iter().any(is_complex_argument)
    }

    fn parens(self, text: &str) -> Option<(u32, u32)> {
        match self {
            Self::Call(call) => call_parens(text, call, unwrap_expression(&call.callee).span().end),
            Self::New(new) => {
                let type_arguments_end = new.type_arguments.as_ref().map_or(0, |args| args.span.end);

                argument_parens(text, new.span, new.callee.span().end.max(type_arguments_end))
            }
        }
    }
}

/// Collects candidate calls and template-literal spans in one traversal.
struct Collector<'a> {
    ancestors: Vec<AstKind<'a>>,
    calls: Vec<Invocation<'a>>,
    templates: Vec<Span>,
}

impl Collector<'_> {
    /// A call inside an argument of the nearest enclosing call, when that call
    /// is not expanded itself, keeps its layout. Functions end the search.
    fn nested_in_unexpanded_argument(&self, call: Invocation<'_>) -> bool {
        let span = call.span();

        for ancestor in self.ancestors.iter().rev() {
            if matches!(ancestor, AstKind::Function(_) | AstKind::ArrowFunctionExpression(_)) {
                return false;
            }

            if let Some(outer) = Invocation::of_kind(*ancestor) {
                let inside = outer.arguments().iter().any(|argument| {
                    let outer_span = argument.span();

                    outer_span.start <= span.start && span.end <= outer_span.end
                });

                return inside && !outer.should_expand();
            }
        }

        false
    }
}

impl<'a> Visit<'a> for Collector<'a> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        if let AstKind::TemplateLiteral(template) = kind {
            self.templates.push(template.span);
        } else if let Some(call) = Invocation::of_kind(kind)
            && call.should_expand()
            && !self.nested_in_unexpanded_argument(call)
        {
            self.calls.push(call);
        }

        self.ancestors.push(kind);
    }

    fn leave_node(&mut self, _kind: AstKind<'a>) {
        self.ancestors.pop();
    }
}

struct Writer<'t> {
    text: &'t str,
    unit: &'t str,
    comments: &'t [Comment],
    templates: &'t [Span],
}

impl Writer<'_> {
    fn slice(&self, span: Span) -> &str {
        &self.text[span.start as usize..span.end as usize]
    }

    /// `(\n<arg>,\n<arg>,\n<indent>)` with arguments one unit deeper than `indent`.
    fn call_parens(&self, call: Invocation<'_>, indent: &str) -> Option<String> {
        let (open, close) = call.parens(self.text)?;
        let arguments = call.arguments();

        if arguments.is_empty() || has_comment_between(self.comments, open, close) || !call.should_expand() {
            return None;
        }

        let argument_indent = format!("{indent}{}", self.unit);
        let separator = format!(",\n{argument_indent}");
        let rendered: Vec<String> = arguments.iter().map(|argument| self.node(argument, &argument_indent)).collect();
        let trailing = if matches!(arguments.last(), Some(Argument::SpreadElement(_))) { "" } else { "," };

        Some(format!("(\n{argument_indent}{}{trailing}\n{indent})", rendered.join(&separator)))
    }

    /// An argument placed at `indent`: an expandable call is expanded in turn,
    /// anything else is re-indented from the depth its text came from.
    fn node(&self, argument: &Argument<'_>, indent: &str) -> String {
        if let Some(call) = Invocation::of_argument(argument)
            && call.should_expand()
        {
            if let Some(parens) = self.call_parens(call, indent)
                && let Some((open, _)) = call.parens(self.text)
            {
                return format!("{}{parens}", &self.text[call.span().start as usize..open as usize]);
            }

            return self.rebase(call.span(), indent);
        }

        self.rebase(argument.span(), indent)
    }

    fn in_template(&self, offset: usize) -> bool {
        self.templates.iter().any(|span| (span.start as usize) < offset && offset < span.end as usize)
    }

    /// Move a node's continuation lines from its own line indent to `to`.
    /// Template-literal content keeps its whitespace, which is string data.
    fn rebase(&self, span: Span, to: &str) -> String {
        let text = self.slice(span);
        let from = line_indent(self.text, span.start);

        if from == to || !text.contains('\n') {
            return text.to_owned();
        }

        let mut out = String::with_capacity(text.len() + 16);
        let mut line_start = span.start as usize;

        for (index, line) in text.split('\n').enumerate() {
            if index > 0 {
                out.push('\n');

                // Template content is kept byte for byte; a blank line loses its indent.
                if self.in_template(line_start) {
                    out.push_str(line);
                } else if let Some(rest) = line.strip_prefix(from).filter(|_| !line.trim().is_empty()) {
                    out.push_str(to);
                    out.push_str(rest);
                } else if !line.trim().is_empty() {
                    out.push_str(line);
                }
            } else {
                out.push_str(line);
            }

            line_start += line.len() + 1;
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use oxc_allocator::Allocator;
    use oxc_ast_visit::Visit;
    use oxc_span::SourceType;

    use super::super::tests::apply_once;
    use super::Collector;
    use crate::passes::Pass;
    use crate::syntax::parse;

    fn lines(lines: &[&str]) -> String {
        lines.join("\n")
    }

    fn format(input: &str) -> String {
        apply_once(Pass::ExpandedCall, "fixture.ts", input)
    }

    /// The source between the first and last backtick: a template's literal bytes.
    fn template_body(source: &str) -> &str {
        &source[source.find('`').expect("a template") + 1..source.rfind('`').expect("a template")]
    }

    fn assert_parses(source: &str) {
        assert!(parse(&Allocator::default(), source, SourceType::ts()).is_ok(), "formatted output must still parse: {source}");
    }

    #[test]
    fn expands_a_returned_cast_wrapper_around_a_nested_call_argument() {
        let input =
            lines(&["export function createAuth() {", "\treturn betterAuth(buildAuthConfig(env, db, mailer, app, options)) as unknown as SasuAuth;", "}", ""]);
        let expected = lines(&[
            "export function createAuth() {",
            "\treturn betterAuth(",
            "\t\tbuildAuthConfig(env, db, mailer, app, options),",
            "\t) as unknown as SasuAuth;",
            "}",
            "",
        ]);

        assert_eq!(format(&input), expected);
    }

    #[test]
    fn expands_a_baseline_indented_call_one_unit_past_its_baseline() {
        let input = lines(&["\t\t\tconst value = resolveConfig(prefix, buildOptions(env), { strict: true });", ""]);
        let expected =
            lines(&["\t\t\tconst value = resolveConfig(", "\t\t\t\tprefix,", "\t\t\t\tbuildOptions(env),", "\t\t\t\t{ strict: true },", "\t\t\t);", ""]);
        let output = format(&input);

        assert_eq!(output, expected);
        assert!(!output.contains("\t\t\t\t\t"), "arguments must be base plus one unit, never doubled");
    }

    #[test]
    fn expands_multi_argument_calls_when_one_argument_is_complex() {
        let input = lines(&["const value = resolveConfig(prefix, buildOptions(env), { strict: true });", ""]);
        let expected = lines(&["const value = resolveConfig(", "\tprefix,", "\tbuildOptions(env),", "\t{ strict: true },", ");", ""]);

        assert_eq!(format(&input), expected);
    }

    #[test]
    fn formats_nested_complex_calls_in_one_stable_run() {
        let input = lines(&["function run() {", "\treturn outer(inner(deep(a, b)), tail);", "}", ""]);
        let expected = lines(&["function run() {", "\treturn outer(", "\t\tinner(", "\t\t\tdeep(a, b),", "\t\t),", "\t\ttail,", "\t);", "}", ""]);
        let once = format(&input);

        assert_eq!(once, expected);
        assert_eq!(format(&once), expected);
    }

    #[test]
    fn expands_object_and_array_arguments_without_rewriting_their_internals() {
        let input = lines(&["const config = createConfig({ hooks: [init(), done] }, [first(), second]);", ""]);
        let expected = lines(&["const config = createConfig(", "\t{ hooks: [init(), done] },", "\t[first(), second],", ");", ""]);

        assert_eq!(format(&input), expected);
    }

    #[test]
    fn expands_a_new_expression_whose_conditional_argument_holds_a_call() {
        let input = lines(&[
            "function link() {",
            "\treturn new DirectAppLink(configured.isPresent() ? configured : runtime.passkeyConfig().map((config) => `https://${config.relyingPartyId}`), runtime.isLocal());",
            "}",
            "",
        ]);
        let expected = lines(&[
            "function link() {",
            "\treturn new DirectAppLink(",
            "\t\tconfigured.isPresent() ? configured : runtime.passkeyConfig().map((config) => `https://${config.relyingPartyId}`),",
            "\t\truntime.isLocal(),",
            "\t);",
            "}",
            "",
        ]);
        let once = format(&input);

        assert_eq!(once, expected);
        assert_eq!(format(&once), once);
    }

    #[test]
    fn expands_a_generic_new_expression_and_a_new_argument_of_an_expanded_call() {
        let input = lines(&["const index = new Map<string, number>(Object.entries(source));", "const app = create(new Router(routes()), options());", ""]);
        let expected = lines(&[
            "const index = new Map<string, number>(",
            "\tObject.entries(source),",
            ");",
            "const app = create(",
            "\tnew Router(",
            "\t\troutes(),",
            "\t),",
            "\toptions(),",
            ");",
            "",
        ]);

        assert_eq!(format(&input), expected);
    }

    #[test]
    fn leaves_simple_new_expressions_and_plain_conditionals_alone() {
        let input = lines(&[
            "const error = new Error(`failed: ${reason}`);",
            "const plain = new Plain();",
            "const bare = new Bare;",
            "const wrapper = new Wrapper(new Inner(a, b), c);",
            "const classes = cn('base', active ? 'on' : 'off');",
            "",
        ]);

        assert_eq!(format(&input), input);
    }

    #[test]
    fn expands_a_call_whose_nested_conditional_branch_holds_an_object() {
        let input = lines(&["const value = pick(a ? b : c ? { d } : e, f);", ""]);
        let expected = lines(&["const value = pick(", "\ta ? b : c ? { d } : e,", "\tf,", ");", ""]);

        assert_eq!(format(&input), expected);
    }

    #[test]
    fn skips_calls_with_comments_inside_the_argument_list() {
        let input = lines(&["const value = createConfig(/* keep inline */ buildOptions(env));", ""]);

        assert_eq!(format(&input), input);
    }

    #[test]
    fn does_not_add_a_trailing_comma_after_a_final_spread_argument() {
        let input = lines(&["const result = invoke(buildOptions(env), ...args);", ""]);
        let expected = lines(&["const result = invoke(", "\tbuildOptions(env),", "\t...args", ");", ""]);

        assert_eq!(format(&input), expected);
    }

    #[test]
    fn leaves_simple_transform_chains_and_computed_callbacks_unchanged() {
        let input = lines(&["const normalized = value.trim().toLowerCase();", "const item = computed(() => value);", ""]);

        assert_eq!(format(&input), input);
    }

    #[test]
    fn leaves_method_chain_arguments_for_fluent_formatters() {
        let input = lines(&["const rows = builder.where(and(eq(users.id, id), eq(users.active, true)));", ""]);

        assert_eq!(format(&input), input);
    }

    #[test]
    fn skips_declaration_files() {
        let input = lines(&["declare const value: ReturnType<typeof createConfig>;", ""]);

        assert_eq!(apply_once(Pass::ExpandedCall, "types.d.ts", &input), input);
    }

    #[test]
    fn nests_with_four_spaces_when_the_source_is_space_indented() {
        let input = lines(&["function run() {", "    return outer(inner(deep(a, b)), tail);", "}", ""]);
        let expected =
            lines(&["function run() {", "    return outer(", "        inner(", "            deep(a, b),", "        ),", "        tail,", "    );", "}", ""]);
        let once = format(&input);

        assert_eq!(once, expected);
        assert_eq!(format(&once), expected);
        assert!(!once.contains('\t'), "expanded output must not introduce tabs into a space-indented file");
    }

    #[test]
    fn nests_a_space_indented_top_level_call_one_level_deep_with_spaces() {
        let input = lines(&["export default defineConfig({", "    cacheDir: \"../../storage/.cache\",", "    test: { globals: true },", "});", ""]);
        let output = format(&input);

        assert!(!output.contains('\t'), "space-indented expansion must not introduce tabs");
        assert!(output.contains("defineConfig(\n    {"), "{output}");
    }

    #[test]
    fn re_indents_a_multiline_object_argument_to_its_new_depth() {
        let input = lines(&["function send() {", "\tconst response = fetch(url, {", "\t\t...init,", "\t\theaders,", "\t});", "}", ""]);
        let expected =
            lines(&["function send() {", "\tconst response = fetch(", "\t\turl,", "\t\t{", "\t\t\t...init,", "\t\t\theaders,", "\t\t},", "\t);", "}", ""]);
        let once = format(&input);

        assert_eq!(once, expected);
        assert_eq!(format(&once), expected);
    }

    #[test]
    fn never_re_indents_the_interior_of_a_multiline_template_literal() {
        let input = lines(&[
            "const Harness = defineComponent({",
            "    template: `",
            "        <div>",
            "            <span>hello</span>",
            "        </div>",
            "    `,",
            "});",
            "",
        ]);
        let expected = lines(&[
            "const Harness = defineComponent(",
            "    {",
            "        template: `",
            "        <div>",
            "            <span>hello</span>",
            "        </div>",
            "    `,",
            "    },",
            ");",
            "",
        ]);
        let once = format(&input);

        assert_eq!(once, expected);
        assert_eq!(format(&once), expected);
    }

    #[test]
    fn preserves_whitespace_only_lines_inside_a_template_literal() {
        let input = lines(&["const query = run(build(), {", "\tsql: `", "\t\tselect 1", "   ", "\t\tfrom t", "\t`,", "});", ""]);
        let output = format(&input);

        assert!(output.contains("\n   \n"), "a blank line carrying string content must keep its bytes");
        assert_eq!(format(&output), output);
    }

    #[test]
    fn reaches_a_fixed_point_for_a_tab_indented_template_literal() {
        let input = lines(&["const Harness = defineComponent({", "\ttemplate: `", "\t\t<div>", "\t\t\t<span>hello</span>", "\t\t</div>", "\t`,", "});", ""]);
        let once = format(&input);

        assert_ne!(once, input, "the call should have expanded");
        assert_eq!(format(&once), once);
        assert_eq!(template_body(&once), template_body(&input), "template interior bytes must be preserved");
    }

    #[test]
    fn reaches_a_fixed_point_for_a_template_literal_with_multiline_interpolations() {
        let input = lines(&[
            "const q = build({",
            "    query: `",
            "        SELECT *",
            "        FROM ${",
            "            resolveTable(schema)",
            "        }",
            "        WHERE id = ${id}",
            "    `,",
            "});",
            "",
        ]);
        let once = format(&input);

        assert_ne!(once, input, "the call should have expanded");
        assert_eq!(format(&once), once);
        assert_eq!(template_body(&once), template_body(&input), "template interior bytes must be preserved");
    }

    #[test]
    fn reaches_a_fixed_point_for_a_tagged_template_literal() {
        let input = lines(&["const styled = create({", "    styles: css`", "        color: red;", "        margin: ${spacing}px;", "    `,", "});", ""]);
        let once = format(&input);

        assert_ne!(once, input, "the call should have expanded");
        assert_eq!(format(&once), once);
        assert_eq!(template_body(&once), template_body(&input), "template interior bytes must be preserved");
    }

    #[test]
    fn converges_over_a_template_literal_nested_two_expansions_deep() {
        let input = lines(&[
            "const app = createApp({",
            "    root: defineComponent({",
            "        template: `",
            "            <div>deep</div>",
            "        `,",
            "    }),",
            "});",
            "",
        ]);
        let mut current = input.clone();

        for _ in 0..5 {
            let next = format(&current);

            if next == current {
                break;
            }

            current = next;
        }

        assert_ne!(current, input, "both calls should have expanded");
        assert!(current.contains("root: defineComponent(\n"), "the inner call must also expand: {current}");
        assert_eq!(format(&current), current, "the fixed point must be stable");
        assert_eq!(template_body(&current), template_body(&input), "template interior bytes must be preserved");
    }

    #[test]
    fn keeps_a_call_site_type_argument_holding_a_function_type_intact() {
        let input = lines(&[
            "const continueByFrame = useMemo<Readonly<Record<AddMemberFrame, () => void>>>(() => ({ type: () => {}, details: () => {} }), [a, b]);",
            "",
        ]);
        let expected = lines(&[
            "const continueByFrame = useMemo<Readonly<Record<AddMemberFrame, () => void>>>(",
            "\t() => ({ type: () => {}, details: () => {} }),",
            "\t[a, b],",
            ");",
            "",
        ]);
        let once = format(&input);

        assert_eq!(once, expected);
        assert_parses(&once);
        assert_eq!(format(&once), once);
    }

    #[test]
    fn keeps_a_bare_arrow_type_argument_intact() {
        let input = lines(&["const run = useCallback<(x: number) => void>(() => ({ a: 1 }), [a]);", ""]);
        let expected = lines(&["const run = useCallback<(x: number) => void>(", "\t() => ({ a: 1 }),", "\t[a],", ");", ""]);
        let once = format(&input);

        assert_eq!(once, expected);
        assert_parses(&once);
    }

    #[test]
    fn keeps_a_nested_generic_type_argument_holding_a_function_type_intact() {
        let input = lines(&["const registry = build<Map<string, () => Promise<void>>>({ a: 1 }, [b]);", ""]);
        let expected = lines(&["const registry = build<Map<string, () => Promise<void>>>(", "\t{ a: 1 },", "\t[b],", ");", ""]);
        let once = format(&input);

        assert_eq!(once, expected);
        assert_parses(&once);
    }

    #[test]
    fn keeps_a_plain_calls_function_type_argument_intact() {
        let input = lines(&["foo<() => void>({ a: 1 }, [b]);", ""]);
        let expected = lines(&["foo<() => void>(", "\t{ a: 1 },", "\t[b],", ");", ""]);
        let once = format(&input);

        assert_eq!(once, expected);
        assert_parses(&once);
    }

    #[test]
    fn expands_a_call_whose_type_arguments_carry_no_parenthesis() {
        let input = lines(&["const store = create<StoreShape>({ a: 1 }, [b]);", ""]);
        let expected = lines(&["const store = create<StoreShape>(", "\t{ a: 1 },", "\t[b],", ");", ""]);

        assert_eq!(format(&input), expected);
    }

    /// Ported from v1 `TemplateSpans`: whether an offset is inside a template literal.
    fn template_contains(source: &str, offset: usize) -> bool {
        let allocator = Allocator::default();
        let program = parse(&allocator, source, SourceType::ts()).expect("parses");
        let mut collector = Collector { ancestors: Vec::new(), calls: Vec::new(), templates: Vec::new() };

        collector.visit_program(program);

        collector.templates.iter().any(|span| (span.start as usize) < offset && offset < span.end as usize)
    }

    #[test]
    fn template_spans_cover_a_literal_interior_but_not_the_line_that_opens_it() {
        let source = lines(&["const markup = `", "\t<div>", "\t\t<span>hello</span>", "\t</div>", "`;", ""]);

        assert!(!template_contains(&source, source.find("const").unwrap()));
        assert!(!template_contains(&source, source.find('`').unwrap()));
        assert!(template_contains(&source, source.find("\t<div>").unwrap()));
        assert!(template_contains(&source, source.find("\t\t<span>").unwrap()));
        // The closing backtick still carries the literal's last line.
        assert!(template_contains(&source, source.rfind('`').unwrap()));
        assert!(!template_contains(&source, source.len() - 1));
    }

    #[test]
    fn template_spans_cover_tagged_nested_and_interpolated_literals() {
        let source = lines(&["const query = sql`", "\tselect ${column(`", "\t\tinner", "\t`)}", "`;", ""]);

        assert!(template_contains(&source, source.find("\tselect").unwrap()));
        assert!(template_contains(&source, source.find("\t\tinner").unwrap()));
    }

    #[test]
    fn template_spans_are_empty_for_source_without_a_template_literal() {
        assert!(!template_contains("const value = 'plain';\n", 5));
    }
}
