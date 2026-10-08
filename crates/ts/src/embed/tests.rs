//! Embedded-language templates through the whole pipeline, after oxfmt's
//! `test/api/embedded_languages.test.ts`: each is formatted, and a second run
//! is a fixed point.

use fmtkit_config::TsFormat;
use fmtkit_core::Lang;

use crate::format_source;

/// The pipeline output for `source`, checked to be a fixed point.
fn format(rel: &str, source: &str) -> String {
    let lang = Lang::from_path(std::path::Path::new(rel)).expect("a script path");
    let once = format_source(rel, lang, source, &TsFormat::default(), false).unwrap_or_else(|error| panic!("{error}\n{source}"));
    let twice = format_source(rel, lang, &once.output, &TsFormat::default(), false).unwrap_or_else(|error| panic!("{error}\n{}", once.output));

    assert_eq!(twice.output, once.output, "a second run changed\n{}", once.output);
    assert!(twice.applied.is_empty(), "{:?}", twice.applied);

    once.output
}

fn lines(lines: &[&str]) -> String {
    let mut text = lines.join("\n");

    text.push('\n');

    text
}

#[test]
fn formats_css_in_js_tags_props_and_styled_jsx() {
    let source = "const button = styled.button`color:${(p) => p.color};&:hover{color:red}`;\nconst global = css.global`.reset{margin:0;padding:0}`;\n";

    assert_eq!(
        format("a.ts", source),
        lines(&[
            "const button = styled.button`",
            "\tcolor: ${(p) => p.color};",
            "\t&:hover {",
            "\t\tcolor: red;",
            "\t}",
            "`;",
            "const global = css.global`",
            "\t.reset {",
            "\t\tmargin: 0;",
            "\t\tpadding: 0;",
            "\t}",
            "`;",
        ])
    );

    assert_eq!(
        format("a.tsx", "const v = <div css={`display:flex;align-items:center`}>Hi</div>;\n"),
        lines(&["const v = (", "\t<div", "\t\tcss={`", "\t\t\tdisplay: flex;", "\t\t\talign-items: center;", "\t\t`}", "\t>", "\t\tHi", "\t</div>", ");"])
    );
}

#[test]
fn formats_graphql_tags_calls_and_comments() {
    let source =
        "const query = gql`query GetUser($id:ID!){user(id:$id){name email}}`;\nexport default graphql(`{users{name}}`);\nconst tagged = /* GraphQL */ `{a}`;\n";

    assert_eq!(
        format("a.ts", source),
        lines(&[
            "const query = gql`",
            "\tquery GetUser($id: ID!) {",
            "\t\tuser(id: $id) {",
            "\t\t\tname",
            "\t\t\temail",
            "\t\t}",
            "\t}",
            "`;",
            "",
            "export default graphql(`",
            "\t{",
            "\t\tusers {",
            "\t\t\tname",
            "\t\t}",
            "\t}",
            "`);",
            "",
            "const tagged = /* GraphQL */ `",
            "\t{",
            "\t\ta",
            "\t}",
            "`;",
        ])
    );
}

#[test]
fn formats_html_templates_with_their_expressions() {
    let source = "const view = html`\n<ul><li>${first}</li>\n\n<li>two</li></ul>\n`;\nconst block = html`<div class=container><h1>${title}</h1><p>World</p></div>`;\nconst roots = html`<h1>${title}</h1>\n<ul><li>a</li><li>b</li></ul>`;\n";

    assert_eq!(
        format("a.ts", source),
        lines(&[
            "const view = html`",
            "\t<ul>",
            "\t\t<li>${first}</li>",
            "",
            "\t\t<li>two</li>",
            "\t</ul>",
            "`;",
            "",
            "const block = html`<div class=\"container\">",
            "\t<h1>${title}</h1>",
            "\t<p>World</p>",
            "</div>`;",
            "",
            "const roots = html`<h1>${title}</h1>",
            "\t<ul>",
            "\t\t<li>a</li>",
            "\t\t<li>b</li>",
            "\t</ul>`;",
        ])
    );
}

#[test]
fn formats_markdown_templates_with_tilde_fences() {
    let source = "function docs() {\n\treturn md`\n\t\t# Title\n\t\t* one\n\t\t* two\n\n\t\t\\`\\`\\`ts\n\t\tconst a = 1\n\t\t\\`\\`\\`\n\t`;\n}\n";

    assert_eq!(
        format("a.ts", source),
        lines(&["function docs() {", "\treturn md`", "\t\t# Title", "", "\t\t- one", "\t\t- two", "", "\t\t~~~ts", "\t\tconst a = 1", "\t\t~~~", "\t`;", "}"])
    );
}

#[test]
fn formats_angular_styles_and_keeps_its_template() {
    let source = "@Component({ template: `<h1>{{    title    }}</h1>`, styles: [`h1{color:blue}`] })\nclass App {}\n";
    let output = format("a.ts", source);

    assert!(output.contains("template: `<h1>{{    title    }}</h1>`,"), "{output}");
    assert!(output.contains("\t\t\th1 {\n\t\t\t\tcolor: blue;\n\t\t\t}\n"), "{output}");
}

#[test]
fn keeps_malformed_and_unsupported_templates_as_written() {
    let source = "const broken = css`a { color: red`;\nconst unclosed = gql`query {`;\nconst other = sql`select   1`;\nconst plain = `a{color:red}`;\n";

    assert_eq!(format("a.ts", source), source);
}
