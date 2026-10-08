//! Routing and policy tests with a stand-in script formatter, so they hold
//! whatever the TS pipeline prints. Goldens through the real pipeline live in
//! `tests/goldens.rs`.

use std::cell::RefCell;

use proptest::prelude::*;

use super::*;

/// Collapses runs of whitespace, drops blank lines and ends with `\n`, which
/// is idempotent. A block containing `BROKEN` fails on that line.
fn tidy(lang: Lang, code: &str, _: &TsFormat) -> Result<String, TsError> {
    if let Some(index) = code.find("BROKEN") {
        let line = u32::try_from(code[..index].matches('\n').count()).unwrap_or(0) + 1;

        return Err(TsError::Syntax { line, column: 1, message: format!("{lang:?} does not parse") });
    }

    let mut out = String::new();

    for line in code.lines().map(|line| line.split_whitespace().collect::<Vec<_>>().join(" ")).filter(|line| !line.is_empty()) {
        out.push_str(&line);
        out.push('\n');
    }

    Ok(out)
}

/// Runs [`tidy`] and records every call of one formatting round.
#[derive(Default)]
struct Recorder {
    calls: RefCell<Vec<(Lang, String, bool)>>,
}

impl Recorder {
    fn format(&self, lang: Lang, source: &str) -> Result<Formatted, HostError> {
        let script = |lang: Lang, code: &str, options: &TsFormat| {
            self.calls.borrow_mut().push((lang, code.to_owned(), options.single_quote));

            tidy(lang, code, options)
        };

        round(lang, source, &TsFormat::default(), &script)
    }

    fn blocks(&self) -> Vec<(Lang, String)> {
        self.calls.borrow().iter().map(|(lang, code, _)| (*lang, code.clone())).collect()
    }
}

fn format(lang: Lang, source: &str) -> Formatted {
    format_with(lang, source, &TsFormat::default(), &tidy).unwrap_or_else(|error| panic!("{error}\n{source}"))
}

fn assert_idempotent(lang: Lang, source: &str) {
    let once = format(lang, source).output;
    let twice = format(lang, &once).output;

    assert_eq!(twice, once, "not idempotent for {lang:?}:\n{source}");
}

// Vue

#[test]
fn vue_script_setup_and_style_are_routed() {
    let source =
        "<script setup lang=\"ts\">\nconst   n = 1\n</script>\n\n<template>\n  <p>{{  n  }}</p>\n</template>\n\n<style scoped>\n.a{color:red}\n</style>\n";
    let recorder = Recorder::default();
    let formatted = recorder.format(Lang::Vue, source).unwrap();

    assert_eq!(
        formatted.output,
        "<script setup lang=\"ts\">\nconst n = 1\n</script>\n\n<template>\n\t<p>{{ n }}</p>\n</template>\n\n<style scoped>\n.a {\n\tcolor: red;\n}\n</style>\n"
    );
    assert_eq!(formatted.applied, ["embedded", "css", "markup"]);
    assert_eq!(recorder.blocks()[0], (Lang::Ts, "\nconst   n = 1\n".to_owned()));
}

#[test]
fn vue_script_lang_selects_the_dialect() {
    for (tag, lang) in [("", Lang::Js), (" lang=\"tsx\"", Lang::Tsx), (" lang=\"jsx\"", Lang::Jsx), (" lang=\"ts\"", Lang::Ts), (" LANG='TS'", Lang::Ts)] {
        let recorder = Recorder::default();

        recorder.format(Lang::Vue, &format!("<script{tag}>\nconst n = 1;\n</script>\n")).unwrap();

        assert_eq!(recorder.blocks(), [(lang, "\nconst n = 1;\n".to_owned())], "{tag}");
    }
}

#[test]
fn vue_non_script_languages_are_left_alone() {
    let source = "<script lang=\"coffee\">\nx  =  1\n</script>\n\n<i18n lang=\"json\">\n{\"a\":  1}\n</i18n>\n";
    let recorder = Recorder::default();
    let formatted = recorder.format(Lang::Vue, source).unwrap();

    assert_eq!(recorder.blocks(), []);
    assert!(formatted.output.contains("x  =  1"));
    assert!(formatted.output.contains("{\"a\":  1}"));
}

#[test]
fn vue_template_expressions_prefer_single_quotes_inside_attributes() {
    let recorder = Recorder::default();
    let source = "<template>\n  <a :title=\"x.y\">{{ y }}</a>\n</template>\n";
    let script = |lang: Lang, code: &str, options: &TsFormat| {
        recorder.calls.borrow_mut().push((lang, code.to_owned(), options.single_quote));

        tidy(lang, code, options)
    };
    let options = TsFormat { single_quote: false, ..TsFormat::default() };

    format_with(Lang::Vue, source, &options, &script).unwrap();

    let calls = recorder.calls.borrow();

    assert!(calls.iter().any(|(_, code, single)| code == "x.y" && *single), "{calls:?}");
    assert!(calls.iter().any(|(_, code, single)| code == "y" && !*single), "{calls:?}");
}

#[test]
fn vue_script_failure_fails_the_document_at_the_host_line() {
    let source = "<template>\n  <p />\n</template>\n\n<script lang=\"ts\">\nconst a = 1;\nBROKEN\n</script>\n";
    let error = format_with(Lang::Vue, source, &TsFormat::default(), &tidy).unwrap_err();

    assert_eq!(error, HostError::Embedded { lang: Lang::Ts, line: 7, message: "Ts does not parse".to_owned() });
}

#[test]
fn vue_expression_failure_keeps_the_expression() {
    let source = "<template>\n  <p :title=\"BROKEN  +  1\">{{ BROKEN }}</p>\n</template>\n";
    let formatted = format(Lang::Vue, source);

    assert!(formatted.output.contains(":title=\"BROKEN  +  1\""), "{}", formatted.output);
    assert!(formatted.output.contains("{{ BROKEN }}"), "{}", formatted.output);
}

#[test]
fn vue_markup_syntax_error_reports_one_based_position() {
    let error = format_with(Lang::Vue, "<template>\n  <div>\n</template>\n", &TsFormat::default(), &tidy).unwrap_err();

    let HostError::Syntax { line, column, message } = error else {
        panic!("expected a syntax error, got {error:?}");
    };

    assert!(line >= 2, "{line}:{column} {message}");
    assert!(column >= 1);
    assert!(message.contains("<div>"), "{message}");
}

#[test]
fn vue_unsupported_and_invalid_styles_are_left_alone() {
    let source =
        "<style lang=\"stylus\">\n.a\n  color  red\n</style>\n\n<style lang=\"sass\">\n.a\n  color:  red\n</style>\n\n<style>\n.a { color: red\n</style>\n";
    let formatted = format(Lang::Vue, source);

    assert!(formatted.output.contains("  color  red"));
    assert!(formatted.output.contains("  color:  red"));
    assert!(formatted.output.contains(".a { color: red"));
    assert!(!formatted.applied.contains(&"css"));
}

#[test]
fn vue_raw_blocks_are_kept_verbatim() {
    let source = "<template lang=\"pug\">\ndiv\n  p  hello\n    span x\n</template>\n\n<i18n lang=\"yaml\">\n\nen:\n  hello:   x\n\n</i18n>\n\n<docs>\n# Title\n\n  some   docs\n</docs>\n\n<script lang=\"coffee\">\nf = ->\n  x  =  1\n</script>\n\n<script setup>\nconst   a = 1\n</script>\n\n<style lang=\"stylus\">\n.a\n  color  red\n</style>\n";
    let recorder = Recorder::default();
    let formatted = recorder.format(Lang::Vue, source).unwrap();

    assert_eq!(
        formatted.output,
        "<template lang=\"pug\">\ndiv\n  p  hello\n    span x\n</template>\n\n<i18n lang=\"yaml\">\nen:\n  hello:   x\n</i18n>\n\n<docs>\n# Title\n\n  some   docs\n</docs>\n\n<script lang=\"coffee\">\nf = ->\n  x  =  1\n</script>\n\n<script setup>\nconst a = 1\n</script>\n\n<style lang=\"stylus\">\n.a\n  color  red\n</style>\n"
    );
    assert_eq!(recorder.blocks(), [(Lang::Js, "\nconst   a = 1\n".to_owned())]);
    assert_idempotent(Lang::Vue, source);
}

#[test]
fn vue_script_failure_after_a_raw_block_reports_the_original_line() {
    let source = "<docs>\none\ntwo\nthree\n</docs>\n\n<script>\nBROKEN\n</script>\n";
    let error = format_with(Lang::Vue, source, &TsFormat::default(), &tidy).unwrap_err();

    assert_eq!(error, HostError::Embedded { lang: Lang::Js, line: 8, message: "Js does not parse".to_owned() });
}

#[test]
fn html_raw_styles_keep_their_indentation() {
    let source = "<html><head><style lang=\"stylus\">\n.a\n  color red\n</style></head></html>\n";
    let formatted = format(Lang::Html, source);

    assert!(formatted.output.contains("\n.a\n  color red\n"), "{}", formatted.output);
    assert_idempotent(Lang::Html, source);
}

#[test]
fn vue_scss_and_less_styles_are_formatted() {
    let source = "<style lang=\"scss\">\n.a{.b{color:red}}\n</style>\n\n<style lang=\"less\">\n.a{.b{color:red}}\n</style>\n";
    let formatted = format(Lang::Vue, source);
    let nested = ".a {\n\t.b {\n\t\tcolor: red;\n\t}\n}\n";

    assert_eq!(formatted.output.matches(nested).count(), 2, "{}", formatted.output);
}

// HTML

#[test]
fn html_inline_script_style_and_style_attribute() {
    let source = "<!doctype html>\n<html><head><style>body{margin:0}</style></head><body><p style=\"color:red;margin:0\">hi</p><script>const   x = 1</script></body></html>";
    let recorder = Recorder::default();
    let formatted = recorder.format(Lang::Html, source).unwrap();

    assert_eq!(
        formatted.output,
        "<!DOCTYPE html>\n<html>\n\t<head>\n\t\t<style>\n\t\t\tbody {\n\t\t\t\tmargin: 0;\n\t\t\t}\n\t\t</style>\n\t</head>\n\t<body>\n\t\t<p style=\"color: red; margin: 0\">hi</p>\n\t\t<script>\n\t\t\tconst x = 1\n\t\t</script>\n\t</body>\n</html>\n"
    );
    assert_eq!(recorder.blocks(), [(Lang::Js, "const   x = 1".to_owned())]);
    assert_eq!(formatted.applied, ["css", "embedded", "markup"]);
}

#[test]
fn html_script_types_follow_v1() {
    let source = "<script type=\"module\">\na()\n</script>\n<script type=\"text/javascript\">\nb()\n</script>\n<script type=\"application/ld+json\">\n{\"a\":1}\n</script>\n<script type=\"text/template\">\n<p>x</p>\n</script>\n";
    let recorder = Recorder::default();
    let formatted = recorder.format(Lang::Html, source).unwrap();

    assert_eq!(recorder.blocks(), [(Lang::Mjs, "\na()\n".to_owned()), (Lang::Js, "\nb()\n".to_owned())]);
    assert!(formatted.output.contains("{\"a\":1}"));
}

#[test]
fn html_script_failure_fails_the_document() {
    let error = format_with(Lang::Html, "<p>x</p>\n<script>\nBROKEN\n</script>\n", &TsFormat::default(), &tidy).unwrap_err();

    assert_eq!(error, HostError::Embedded { lang: Lang::Js, line: 3, message: "Js does not parse".to_owned() });
}

// Markdown

#[test]
fn markdown_fences_route_by_info_string() {
    let source = "# Title\n\n```ts\nconst   a = 1\n```\n\n```tsx title=\"x.tsx\" {1}\nconst   b = 2\n```\n\n```JavaScript\nconst   c = 3\n```\n\n```bash\necho   hi\n```\n\n```\nplain   text\n```\n";
    let recorder = Recorder::default();
    let formatted = recorder.format(Lang::Markdown, source).unwrap();

    assert_eq!(recorder.blocks(), [(Lang::Ts, "const   a = 1".to_owned()), (Lang::Tsx, "const   b = 2".to_owned()), (Lang::Js, "const   c = 3".to_owned())]);
    assert_eq!(
        formatted.output,
        "# Title\n\n```ts\nconst a = 1\n```\n\n```tsx title=\"x.tsx\" {1}\nconst b = 2\n```\n\n```JavaScript\nconst c = 3\n```\n\n```bash\necho   hi\n```\n\n```\nplain   text\n```\n"
    );
    assert_eq!(formatted.applied, ["embedded", "markdown"]);
}

#[test]
fn markdown_css_fences_are_formatted() {
    let formatted = format(Lang::Markdown, "```css\na{color:red}\n```\n\n```scss\n.a{.b{color:red}}\n```\n");

    assert_eq!(formatted.output, "```css\na {\n\tcolor: red;\n}\n```\n\n```scss\n.a {\n\t.b {\n\t\tcolor: red;\n\t}\n}\n```\n");
    assert_eq!(formatted.applied, ["css", "markdown"]);
}

#[test]
fn markdown_failing_fences_are_left_as_written() {
    let source = "```ts\nconst   a = BROKEN\n```\n\n```css\na { color: red\n```\n\n```ts\nconst   b = 1\n```\n";
    let formatted = format(Lang::Markdown, source);

    assert_eq!(formatted.output, "```ts\nconst   a = BROKEN\n```\n\n```css\na { color: red\n```\n\n```ts\nconst b = 1\n```\n");
}

#[test]
fn markdown_fences_inside_containers_keep_their_prefix() {
    let source = "- item\n\n  ```ts\n  const   a = 1\n\n  const   b = 2\n  ```\n\n> ```ts\n> const   c = 3\n>\n> const   d = 4\n> ```\n";
    let recorder = Recorder::default();
    let formatted = recorder.format(Lang::Markdown, source).unwrap();

    assert_eq!(recorder.blocks()[0], (Lang::Ts, "const   a = 1\n\nconst   b = 2".to_owned()));
    assert_eq!(recorder.blocks()[1], (Lang::Ts, "const   c = 3\n\nconst   d = 4".to_owned()));
    assert!(formatted.output.contains("\n    ```ts\n    const a = 1\n    const b = 2\n    ```\n"), "{}", formatted.output);
    assert!(formatted.output.contains("> ```ts\n> const c = 3\n> const d = 4\n> ```\n"), "{}", formatted.output);
}

#[test]
fn markdown_multiline_output_keeps_blank_lines_inside_containers() {
    // The stand-in adds blank lines on every call, so only one round is run.
    let script = |_: Lang, code: &str, _: &TsFormat| Ok(code.replace(';', ";\n\n"));
    let formatted = round(Lang::Markdown, "> ```ts\n> a;b;\n> ```\n", &TsFormat::default(), &script).unwrap();

    assert_eq!(formatted.output, "> ```ts\n> a;\n>\n> b;\n> ```\n");
}

#[test]
fn markdown_fence_length_grows_when_code_holds_a_fence() {
    let script = |_: Lang, _: &str, _: &TsFormat| Ok("const s = `\n```\n`;\n".to_owned());
    let formatted = format_with(Lang::Markdown, "```ts\nx\n```\n", &TsFormat::default(), &script).unwrap();

    assert_eq!(formatted.output, "````ts\nconst s = `\n```\n`;\n````\n");
}

#[test]
fn markdown_tilde_unterminated_and_indented_code() {
    let recorder = Recorder::default();
    let formatted = recorder.format(Lang::Markdown, "~~~js\nconst   m = 2\n~~~\n\n    const   x = 1\n").unwrap();

    assert_eq!(recorder.blocks(), [(Lang::Js, "const   m = 2".to_owned())]);
    assert_eq!(formatted.output, "```js\nconst m = 2\n```\n\n    const   x = 1\n");

    let recorder = Recorder::default();
    let formatted = recorder.format(Lang::Markdown, "```ts\nconst   x = 1\n").unwrap();

    assert_eq!(formatted.output, "```ts\nconst x = 1\n```\n");
}

#[test]
fn markdown_front_matter_tables_and_lists() {
    let source = "---\ntitle: Notes\ntags: [a, b]\n---\n# Notes\n\n* one\n* two\n\n|a|b|\n|-|:-:|\n|1|22|\n";
    let formatted = format(Lang::Markdown, source);

    assert_eq!(formatted.output, "---\ntitle: Notes\ntags: [a, b]\n---\n\n# Notes\n\n- one\n- two\n\n| a   |  b  |\n| --- | :-: |\n| 1   | 22  |\n");
    assert_eq!(formatted.applied, ["markdown"]);
}

#[test]
fn markdown_empty_fence_is_skipped() {
    let recorder = Recorder::default();
    let formatted = recorder.format(Lang::Markdown, "```ts\n```\n").unwrap();

    assert_eq!(recorder.blocks(), []);
    assert_eq!(formatted.output, "```ts\n\n```\n");
    assert_eq!(formatted.applied, ["markdown"]);
}

// Line endings and envelopes

#[test]
fn crlf_input_formats_like_lf_input() {
    for (lang, source) in [
        (Lang::Vue, "<script setup>\nconst   a = 1\n</script>\n\n<template>\n  <p>x</p>\n</template>\n"),
        (Lang::Html, "<div>\n<p>x</p>\n</div>\n<script>\nconst   a = 1\n</script>\n"),
        (Lang::Markdown, "# T\n\n```ts\nconst   a = 1\n```\n\n- a\n- b\n"),
    ] {
        let lf = format(lang, source);
        let crlf = format(lang, &source.replace('\n', "\r\n"));

        assert_eq!(crlf.output, lf.output, "{lang:?}");
        assert!(!crlf.output.contains('\r'));
        assert!(crlf.applied.contains(&if lang == Lang::Markdown { "markdown" } else { "markup" }));
    }
}

#[test]
fn missing_trailing_newline_is_added() {
    assert_eq!(format(Lang::Markdown, "# T").output, "# T\n");
    assert_eq!(format(Lang::Html, "<p>x</p>").output, "<p>x</p>\n");
    assert_eq!(format(Lang::Vue, "<template><p>x</p></template>").output, "<template><p>x</p></template>\n");
}

#[test]
fn empty_documents_stay_empty() {
    for lang in [Lang::Vue, Lang::Html, Lang::Markdown] {
        let formatted = format(lang, "");

        assert_eq!(formatted.output, "", "{lang:?}");
        assert_eq!(formatted.applied, Vec::<&str>::new());
    }
}

#[test]
fn formatted_input_reports_no_steps() {
    for (lang, source) in [
        (Lang::Vue, "<script setup lang=\"ts\">\nconst n = 1\n</script>\n\n<template>\n\t<p>{{ n }}</p>\n</template>\n"),
        (Lang::Markdown, "# T\n\n```ts\nconst a = 1\n```\n"),
    ] {
        let formatted = format(lang, source);

        assert_eq!(formatted.output, source);
        assert!(formatted.applied.is_empty(), "{:?}", formatted.applied);
    }
}

#[test]
fn non_host_languages_are_returned_unchanged() {
    let formatted = format(Lang::Ts, "const   a = 1\r\n");

    assert_eq!(formatted, Formatted { output: "const   a = 1\r\n".to_owned(), applied: Vec::new() });
}

#[test]
fn format_host_uses_the_ts_pipeline() {
    let formatted = format_host("a.md", Lang::Markdown, "#  T\n", &TsFormat::default()).unwrap();

    assert_eq!(formatted.output, "# T\n");
}

// Properties, ported from v1's markdown-fences and vue-script property tests:
// script blocks reach the script formatter with their exact content and
// dialect, others never do, and formatting is idempotent.

fn fence_strategy() -> impl Strategy<Value = (String, Option<(Lang, String)>)> {
    let fence = prop::sample::select(vec!["```", "````", "~~~", "~~~~"]);
    let lang = prop::sample::select(vec!["ts", "tsx", "js", "jsx", "typescript", "javascript", "mjs", "json", "bash", "yaml", ""]);
    let line = prop::sample::select(vec!["const value = 1;", "export default {};", "// a comment", "let total = sum(a,  b);"]);

    // The prefix of the fence line and of the lines after it: top level, a
    // list item, a blockquote.
    let container = prop::sample::select(vec![("", ""), ("- ", "  "), ("> ", "> ")]);

    (fence, lang, prop::collection::vec(line, 0..4), container).prop_map(|(fence, lang, lines, (first, rest))| {
        let body = lines.join("\n");
        let mut fenced = format!("intro prose line\n\n{first}{fence}{lang}\n");

        for line in &lines {
            fenced.extend([rest, line, "\n"]);
        }

        fenced.extend([rest, fence, "\n\n"]);

        let routed = match embed::Embedded::from_tag(lang) {
            Some(embed::Embedded::Script(dialect)) if !body.trim().is_empty() => Some((dialect, body)),
            _ => None,
        };

        (fenced, routed)
    })
}

fn vue_block_strategy() -> impl Strategy<Value = (String, Option<Lang>)> {
    let lang = prop::sample::select(vec![
        ("", None),
        ("lang=\"ts\"", Some(Lang::Ts)),
        ("lang=\"tsx\"", Some(Lang::Tsx)),
        ("lang=js", Some(Lang::Js)),
        ("lang=\"json\"", None),
    ]);
    let extra = prop::sample::select(vec!["", "setup", "data-x=\"1\""]);
    let content = prop::sample::select(vec!["const value = 1;\n", "\nexport default {};\n", "// embedded script\n"]);
    let other = prop::sample::select(vec![
        "<template>\n<div>fixture</div>\n</template>",
        "<style scoped>\n.value { color: red; }\n</style>",
        "<style module>\n.a{b:c}\n</style>",
        "<style lang=\"stylus\">\n.a\n  color red\n</style>",
        "<template lang=\"pug\">\ndiv\n  p  hi\n</template>",
        "<i18n lang=\"yaml\">\nen:\n  a:   1\n</i18n>",
        "<script lang=\"coffee\">\nf = ->\n  x  =  1\n</script>",
        "<!-- <script>BROKEN</script> -->",
    ]);

    prop_oneof![
        (lang, extra, content).prop_map(|((attribute, lang), extra, content)| {
            let attributes: Vec<&str> = [attribute, extra].into_iter().filter(|a| !a.is_empty()).collect();
            let open = if attributes.is_empty() { "<script>".to_owned() } else { format!("<script {}>", attributes.join(" ")) };
            let lang = if attribute.is_empty() { Some(Lang::Js) } else { lang };

            (format!("{open}{content}</script>"), lang)
        }),
        other.prop_map(|markup| (markup.to_owned(), None)),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn markdown_fences_reach_the_script_formatter_intact(fences in prop::collection::vec(fence_strategy(), 1..6)) {
        let document: String = fences.iter().map(|(fenced, _)| fenced.as_str()).collect();
        let expected: Vec<(Lang, String)> = fences.iter().filter_map(|(_, routed)| routed.clone()).collect();
        let recorder = Recorder::default();

        recorder.format(Lang::Markdown, &document).unwrap();

        prop_assert_eq!(recorder.blocks(), expected);
        assert_idempotent(Lang::Markdown, &document);
    }

    #[test]
    fn vue_scripts_reach_the_script_formatter_by_dialect(blocks in prop::collection::vec(vue_block_strategy(), 1..6)) {
        let document = blocks.iter().map(|(markup, _)| markup.as_str()).collect::<Vec<_>>().join("\n") + "\n";
        let expected: Vec<Lang> = blocks.iter().filter_map(|(_, lang)| *lang).collect();
        let recorder = Recorder::default();

        recorder.format(Lang::Vue, &document).unwrap();

        prop_assert_eq!(recorder.blocks().into_iter().map(|(lang, _)| lang).collect::<Vec<_>>(), expected);
        assert_idempotent(Lang::Vue, &document);
    }

    #[test]
    fn crlf_and_trailing_newline_never_change_the_result(fences in prop::collection::vec(fence_strategy(), 1..4), crlf: bool, trim: bool) {
        let document: String = fences.iter().map(|(fenced, _)| fenced.as_str()).collect();
        let mut variant = if crlf { document.replace('\n', "\r\n") } else { document.clone() };

        if trim {
            variant.truncate(variant.trim_end().len());
        }

        prop_assert_eq!(format(Lang::Markdown, &variant).output, format(Lang::Markdown, &document).output);
    }
}

#[test]
fn same_code_ignores_common_indent_and_blank_edges() {
    assert!(same_code("\n\t\t\tbody {\n\t\t\t\tmargin: 0;\n\t\t\t}\n\t\t", "body {\n\tmargin: 0;\n}\n"));
    assert!(same_code("\nconst a = 1;\n", "const a = 1;\n"));
    assert!(!same_code("a {\n  b: c;\n}", "a {\n\tb: c;\n}"));
    assert!(!same_code("const a = 1", "const a = 1;"));
}

// Fixed point

#[test]
fn a_formatted_document_takes_one_round() {
    let source = "<script setup lang=\"ts\">\nconst n = 1\n</script>\n\n<template>\n\t<p>{{ n }}</p>\n</template>\n";
    let calls = RefCell::new(0);
    let script = |lang: Lang, code: &str, options: &TsFormat| {
        *calls.borrow_mut() += 1;

        tidy(lang, code, options)
    };
    let formatted = format_with(Lang::Vue, source, &TsFormat::default(), &script).unwrap();

    assert_eq!(formatted.output, source);
    assert_eq!(formatted.applied, Vec::<&str>::new());
    assert_eq!(*calls.borrow(), 2, "one script block and one expression, once each");
}

#[test]
fn a_changed_document_is_formatted_until_it_settles() {
    // Each round strips one leading `!`, so three rounds are needed.
    let script = |_: Lang, code: &str, _: &TsFormat| Ok(code.replacen('!', "", 1));
    let formatted = format_with(Lang::Vue, "<script>\n!!!a\n</script>\n", &TsFormat::default(), &script).unwrap();

    assert_eq!(formatted.output, "<script>\na\n</script>\n");
    assert_eq!(formatted.applied, ["embedded", "markup"]);
}

#[test]
fn a_document_that_never_settles_is_an_idempotency_error() {
    let script = |_: Lang, code: &str, _: &TsFormat| Ok(format!("{code}a\n"));
    let error = format_with(Lang::Vue, "<script>\na\n</script>\n", &TsFormat::default(), &script).unwrap_err();

    assert!(matches!(error, HostError::Invariant { step: "idempotency", .. }), "{error:?}");
}

// Fuzz findings of the `hosts` target, through the real pipeline

fn format_real(lang: Lang, source: &str) -> Result<Formatted, HostError> {
    format_host("fuzz", lang, source, &TsFormat::default())
}

fn assert_settles(lang: Lang, source: &str) -> String {
    let once = format_real(lang, source).unwrap_or_else(|error| panic!("{error}\n{source:?}"));
    let twice = format_real(lang, &once.output).unwrap_or_else(|error| panic!("{error}\n{:?}", once.output));

    assert_eq!(twice.output, once.output, "{lang:?} {source:?}");
    assert!(twice.applied.is_empty(), "{lang:?} {source:?}: {:?}", twice.applied);

    once.output
}

#[test]
fn fuzz_findings_that_settle_within_the_round_budget() {
    assert_eq!(assert_settles(Lang::Html, "<style>a\n \na</style>"), "<style>\n\ta\n\n\ta\n</style>\n");
    assert_eq!(assert_settles(Lang::Vue, "{{a\n \n=}"), "{{\n\ta\n\n\t=\n}}\n");

    for source in ["$$$\n$$", "a\u{c}\r\u{c}", "a\n\n\u{c}", "- `a\r|-\n`", "- a\n  >a\na", ">`a\n--\n`"] {
        assert_settles(Lang::Markdown, source);
    }
}

/// Upstream bugs that never settle; `docs/known-issues.md` lists them. A
/// failure here means the formatter was fixed: update the document.
#[test]
fn fuzz_findings_that_never_settle_are_refused() {
    for (lang, source) in [
        (Lang::Vue, "{{`\n`\n\0}"),
        (Lang::Vue, "{{a\n`\n}}"),
        (Lang::Vue, "{{{;}\n`\n`}"),
        (Lang::Html, "<style>s\n`\nd</style>"),
        (Lang::Markdown, "- a\n\n  {{\n}}"),
        (Lang::Markdown, "- d\n  >`d\n`"),
        (Lang::Markdown, "- ;\n<e>\n\t\t\u{b}"),
        (Lang::Markdown, "- `\n\t\t`\n<m>\n"),
        (Lang::Markdown, "```css\n{r:(}"),
    ] {
        let result = format_real(lang, source);

        assert!(matches!(result, Err(HostError::Invariant { step: "idempotency", .. })), "{lang:?} {source:?}: {result:?}");
    }
}

/// oxc-css-parser 0.0.15 reaches an `unreachable!` on a hyphen that neither
/// starts an identifier nor ends the input; see `docs/known-issues.md`. A
/// failure here means the parser was fixed: update the document.
#[test]
#[should_panic(expected = "entered unreachable code")]
fn a_hyphen_before_a_space_in_a_style_fence_panics_upstream() {
    let _ = format_real(Lang::Markdown, "```css\n.- o");
}
