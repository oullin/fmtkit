//! The routing table and the `FormatDispatcher` installed on every format
//! session, after oxfmt's `embed/dispatcher.rs` and `embed/services.rs`.

use std::sync::Arc;

use fmtkit_config::{TrailingComma, TsFormat};
use oxc_formatter::{CssInJsTemplate, HtmlEmbedMeta, JsFormatOptions};
use oxc_formatter_core::{
    CoreFormatOptions, DispatchPayload, DispatchRequest, DispatchResponse, EmbeddedIr, FormatDispatcher, FormatOptions, FormatSession, LineEnding,
    SessionServices,
};
use oxc_formatter_css::{CssFormatOptions, CssVariant};
use oxc_formatter_graphql::GraphqlFormatOptions;
use oxc_formatter_json::{JsonFormatOptions, JsonVariant};
use oxc_formatter_yaml::YamlFormatOptions;

use super::text::{self, TextOptions};

/// A language with an oxc formatter that builds IR the JS document absorbs.
enum NativeLanguage {
    Graphql,
    /// The request's dialect; css-in-js overrides it with SCSS.
    Css(CssVariant),
    Yaml,
    Json(JsonVariant),
}

/// The languages oxfmt hands to Prettier. HTML and Markdown format to text
/// here instead; Angular has no formatter outside Prettier.
enum TextLanguage {
    Html,
    Angular,
    Markdown,
}

enum Route {
    Native(NativeLanguage),
    Text(TextLanguage),
    Unsupported,
}

/// oxfmt's `route`, including its aliases.
fn route(language: &str) -> Route {
    match language {
        "graphql" | "gql" => Route::Native(NativeLanguage::Graphql),
        "css" => Route::Native(NativeLanguage::Css(CssVariant::Css)),
        "scss" => Route::Native(NativeLanguage::Css(CssVariant::Scss)),
        "less" => Route::Native(NativeLanguage::Css(CssVariant::Less)),
        "yaml" | "yml" => Route::Native(NativeLanguage::Yaml),
        "json" => Route::Native(NativeLanguage::Json(JsonVariant::Json)),
        "jsonc" => Route::Native(NativeLanguage::Json(JsonVariant::Jsonc)),
        "json5" => Route::Native(NativeLanguage::Json(JsonVariant::Json5)),
        "html" => Route::Text(TextLanguage::Html),
        "angular" => Route::Text(TextLanguage::Angular),
        "markdown" | "md" => Route::Text(TextLanguage::Markdown),
        _ => Route::Unsupported,
    }
}

/// The session services for formatting with `options`: the dispatcher and
/// nothing else (no string embedder, see the module docs; no Tailwind sorter,
/// which oxfmt only has under Node).
pub(crate) fn services(options: &TsFormat, js: &JsFormatOptions) -> SessionServices {
    SessionServices { dispatcher: Some(dispatcher(Options::new(options, js))), ..SessionServices::default() }
}

/// Every sub-formatter's options for one `[ts.format]`, mapped the way oxfmt's
/// `to_oxc_formatter_{css,graphql,yaml,json}` map `.oxfmtrc.json`. Keys
/// `[ts.format]` lacks (bracket spacing, prose wrap, quote props, object wrap)
/// keep oxfmt's defaults.
struct Options {
    css: CssFormatOptions,
    graphql: GraphqlFormatOptions,
    yaml: YamlFormatOptions,
    json: JsonFormatOptions,
    text: TextOptions,
}

impl Options {
    fn new(options: &TsFormat, js: &JsFormatOptions) -> Self {
        let core = CoreFormatOptions { indent_style: js.indent_style, indent_width: js.indent_width, line_width: js.line_width, line_ending: LineEnding::Lf };
        let trailing = options.trailing_comma != TrailingComma::None;

        let mut css = CssFormatOptions::default();

        css.apply_core(core);
        css.single_quote = options.single_quote.into();
        css.trailing_commas = if trailing { oxc_formatter_css::TrailingCommas::Always } else { oxc_formatter_css::TrailingCommas::Never };

        let mut graphql = GraphqlFormatOptions::default();

        graphql.apply_core(core);

        let mut yaml = YamlFormatOptions::default();

        yaml.apply_core(core);
        yaml.single_quote = options.single_quote.into();
        yaml.trailing_commas = if trailing { oxc_formatter_yaml::TrailingCommas::Always } else { oxc_formatter_yaml::TrailingCommas::Never };

        let mut json = JsonFormatOptions::default();

        json.apply_core(core);
        json.single_quote = options.single_quote.into();
        json.trailing_commas = if trailing { oxc_formatter_json::TrailingCommas::Always } else { oxc_formatter_json::TrailingCommas::Never };

        Self { css, graphql, yaml, json, text: TextOptions::new(options, js, css) }
    }
}

/// oxfmt's `build_dispatcher` without the Node fallback: native languages
/// format to IR, HTML and Markdown through the text channel, the rest stays.
fn dispatcher(options: Options) -> FormatDispatcher {
    Arc::new(move |session: &FormatSession<'_>, request: DispatchRequest<'_>| {
        let code = request.text;

        Ok(match route(request.language) {
            Route::Native(NativeLanguage::Graphql) => native(oxc_formatter_graphql::format_to_ir(session, code, options.graphql)),
            Route::Native(NativeLanguage::Css(variant)) => {
                // css-in-js parses as SCSS with oxc_formatter's `${}` placeholders.
                let in_js = request.parent_context.is_some_and(|context| context.downcast_ref::<CssInJsTemplate>().is_some());
                let variant = if in_js { CssVariant::Scss } else { variant };

                native(oxc_formatter_css::format_to_ir(session, code, CssFormatOptions { variant, ..options.css }, in_js))
            }
            Route::Native(NativeLanguage::Yaml) => native(oxc_formatter_yaml::format_to_ir(session, code, options.yaml)),
            Route::Native(NativeLanguage::Json(variant)) => {
                native(oxc_formatter_json::format_to_ir(session, code, JsonFormatOptions { variant, ..options.json }))
            }
            Route::Text(TextLanguage::Html) => match text::html(code, &options.text) {
                Some(formatted) => DispatchResponse::Formatted(DispatchPayload {
                    doc: text::into_ir(session, &formatted, options.text.indent_width),
                    tailwind_classes: Vec::new(),
                    child_context: Some(Box::new(HtmlEmbedMeta { has_multiple_root_elements: Some(text::has_multiple_roots(code)) })),
                }),
                None => DispatchResponse::PreserveOriginal,
            },
            Route::Text(TextLanguage::Markdown) => match text::markdown(code, &options.text) {
                Some(formatted) => DispatchResponse::Formatted(DispatchPayload {
                    doc: text::into_ir(session, &formatted, options.text.indent_width),
                    tailwind_classes: Vec::new(),
                    child_context: None,
                }),
                None => DispatchResponse::PreserveOriginal,
            },
            Route::Text(TextLanguage::Angular) | Route::Unsupported => DispatchResponse::PreserveOriginal,
        })
    })
}

/// A native branch's result: a parse failure keeps the template as written.
fn native<E>(result: Result<EmbeddedIr<'_>, E>) -> DispatchResponse<'_> {
    result.map_or(DispatchResponse::PreserveOriginal, |embedded| DispatchResponse::Formatted(embedded.into()))
}

#[cfg(test)]
mod tests {
    use fmtkit_config::TsFormat;
    use oxc_allocator::Allocator;
    use oxc_formatter_core::{CoreFormatOptions, DispatchRequest, FormatOptions, FormatSession, InputKind};

    use super::services;
    use crate::format::js_options;

    /// `text` formatted as `language` and printed on its own; `None` when the
    /// dispatcher keeps it as written.
    fn dispatch(language: &str, text: &str) -> Option<String> {
        let options = TsFormat::default();
        let allocator = Allocator::default();
        let session = FormatSession::with_services(&allocator, InputKind::PhysicalFile, services(&options, &js_options(&options)));
        let request = DispatchRequest { language, text, input_kind: InputKind::Fragment, parent_context: None };

        session.dispatch_to_string(request, CoreFormatOptions::default().as_print_options()).expect("dispatches")
    }

    #[test]
    fn every_routed_language_formats() {
        for (language, text) in [
            ("graphql", "{ a }"),
            ("gql", "{ a }"),
            ("css", "a { color: red }"),
            ("scss", "a { color: red }"),
            ("less", "a { color: red }"),
            ("yaml", "a: 1"),
            ("yml", "a: 1"),
            ("json", "{ \"a\": 1 }"),
            ("jsonc", "{ \"a\": 1 }"),
            ("json5", "{ \"a\": 1 }"),
            ("html", "<div></div>"),
            ("markdown", "# a"),
            ("md", "# a"),
        ] {
            assert!(dispatch(language, text).is_some(), "{language} did not format");
        }
    }

    /// No JS template routes to YAML or JSON directly; they are reached from
    /// other embedded languages, so they are checked here. YAML cannot indent
    /// with tabs, so it nests by the configured width in spaces.
    #[test]
    fn yaml_and_json_format_to_a_fixed_point() {
        for (language, text, expected) in
            [("yaml", "a:   1\nb:\n  - x", "a: 1\nb:\n    - x"), ("json", "{\"a\":[1,2],\"b\":{}}", "{ \"a\": [1, 2], \"b\": {} }")]
        {
            let once = dispatch(language, text).expect("formats");

            assert_eq!(once.trim_end(), expected, "{language}");
            assert_eq!(dispatch(language, &once).as_deref(), Some(once.as_str()), "{language}");
        }
    }

    #[test]
    fn angular_unsupported_and_malformed_requests_stay_as_written() {
        for (language, text) in [("angular", "<div></div>"), ("toml", "a = 1"), ("css", "a { color: red"), ("graphql", "query {"), ("yaml", "a: [")] {
            assert_eq!(dispatch(language, text), None, "{language} should be preserved");
        }
    }
}
