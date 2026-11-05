//! The `naming/*` rules: what each reports, where, and what it leaves alone.

mod support;

use fmtkit_config::Lint;
use fmtkit_lint::Linter;
use serde_json::{Value, json};
use support::{lint, only};

const RULE: &str = "naming/no-redundant-name-prefix";

fn options(extra: Value) -> Vec<Value> {
    let mut object = json!({ "workspaces": ["packages/*", "identity/auth"] });

    if let (Some(object), Value::Object(extra)) = (object.as_object_mut(), extra) {
        object.extend(extra);
    }

    vec![object]
}

fn messages(extra: Value, file: &str, code: &str) -> Vec<String> {
    let result = lint(&only(RULE, &options(extra)), file, code, false);

    assert!(result.diagnostics.iter().all(|d| d.rule == RULE), "{:?}", result.diagnostics);
    result.diagnostics.into_iter().map(|d| d.message).collect()
}

fn assert_clean(extra: Value, file: &str, code: &str) {
    assert_eq!(messages(extra, file, code), Vec::<String>::new(), "{file}");
}

/// Q15: a file name that starts with its workspace's name, in either number.
#[test]
fn file_names_that_repeat_the_workspace_are_reported() {
    assert_eq!(
        messages(json!({}), "packages/twilio/src/twilio-verify-driver.ts", ""),
        ["file name twilio-verify-driver.ts repeats \"twilio\" from its path; name it \"verify-driver.ts\""]
    );
    assert_eq!(
        messages(json!({}), "packages/notifications/src/notification-sink.test.ts", ""),
        ["file name notification-sink.test.ts repeats \"notification\" from its path; name it \"sink.test.ts\""]
    );
    assert_eq!(
        messages(json!({}), "packages/policies/src/policy-table.ts", ""),
        ["file name policy-table.ts repeats \"policy\" from its path; name it \"table.ts\""]
    );
}

/// A plural whose singular keeps the `e` before its `s` is matched in the
/// singular too.
#[test]
fn plurals_that_keep_their_e_are_matched_in_the_singular() {
    assert_eq!(
        messages(json!({}), "packages/caches/src/cache-store.ts", ""),
        ["file name cache-store.ts repeats \"cache\" from its path; name it \"store.ts\""]
    );
    assert_eq!(messages(json!({}), "packages/sizes/src/size-chart.ts", ""), ["file name size-chart.ts repeats \"size\" from its path; name it \"chart.ts\""]);
    assert_eq!(messages(json!({}), "packages/cookies/src/cookie-jar.ts", ""), ["file name cookie-jar.ts repeats \"cookie\" from its path; name it \"jar.ts\""]);
}

/// Q15: a file name that starts with an ancestor folder's name, nested
/// workspaces judged by the innermost one, and consecutive prefixes trimmed
/// together.
#[test]
fn file_names_that_repeat_a_folder_are_reported() {
    assert_eq!(
        messages(json!({}), "identity/auth/src/sign-in/magic-link/magic-link-throttle.ts", ""),
        ["file name magic-link-throttle.ts repeats \"magic-link\" from its path; name it \"throttle.ts\""]
    );
    assert_eq!(
        messages(json!({}), "identity/auth/src/sign-in/auth-sign-in-limit.ts", ""),
        ["file name auth-sign-in-limit.ts repeats \"auth-sign-in\" from its path; name it \"limit.ts\""]
    );
}

/// Q15: a name that is only the prefix says something on its own, a prefix
/// must match whole words, and files outside every workspace are not judged.
#[test]
fn single_clear_words_and_partial_words_are_kept() {
    assert_clean(json!({}), "packages/providers/src/provider.ts", "");
    assert_clean(json!({}), "packages/providers/src/route.ts", "");
    assert_clean(json!({}), "packages/access/src/accessor.ts", "");
    assert_clean(json!({}), "packages/http/src/https-agent.ts", "");
    assert_clean(json!({}), "tools/tools-runner.ts", "");
}

/// A folder is judged against the workspace and the folders above it.
#[test]
fn folder_names_are_judged_against_the_parent_path() {
    assert_eq!(
        messages(json!({}), "packages/notifications/src/notification-channels/push.ts", ""),
        ["folder packages/notifications/src/notification-channels/ repeats \"notification\" from its path; name it \"channels\""]
    );
    assert_eq!(
        messages(json!({}), "identity/auth/src/sign-in/sign-in-phone/code.ts", ""),
        ["folder identity/auth/src/sign-in/sign-in-phone/ repeats \"sign-in\" from its path; name it \"phone\""]
    );
    assert_clean(json!({}), "identity/auth/src/sign-in/phone-verification/code.ts", "");
}

/// Q14: module-local declarations are judged against the whole path, exported
/// ones against the workspace only.
#[test]
fn declarations_are_reported_by_visibility() {
    let code = "\
const magicLinkTtl = 5;
function signInAttempt() {}
type MagicLinkState = string;
export class MagicLinkSender {}
export const AuthMagicLinkLimit = 3;
const authRetryWindow = 1;
export { authRetryWindow };
";

    assert_eq!(
        messages(json!({}), "identity/auth/src/sign-in/magic-link/sender.ts", code),
        [
            "\"magicLinkTtl\" repeats \"magic-link\" from its path; name it \"ttl\"",
            "\"signInAttempt\" repeats \"sign-in\" from its path; name it \"attempt\"",
            "\"MagicLinkState\" repeats \"magic-link\" from its path; name it \"State\"",
            "\"AuthMagicLinkLimit\" repeats \"auth\" from its workspace name; name it \"MagicLinkLimit\"",
            "\"authRetryWindow\" repeats \"auth\" from its workspace name; name it \"retryWindow\"",
        ]
    );
}

/// Q14: `TwilioLookupDriver` becomes `LookupDriver`; `AccessProvider` keeps
/// its prefix because `Provider` alone would clash where it is imported.
#[test]
fn exported_names_keep_a_prefix_that_would_leave_one_word() {
    assert_eq!(
        messages(json!({}), "packages/twilio/src/lookup-driver.ts", "export class TwilioLookupDriver {}\nexport const TWILIO_TIMEOUT_MS = 1;\n"),
        [
            "\"TwilioLookupDriver\" repeats \"twilio\" from its workspace name; name it \"LookupDriver\"",
            "\"TWILIO_TIMEOUT_MS\" repeats \"twilio\" from its workspace name; name it \"TIMEOUT_MS\"",
        ]
    );
    assert_clean(json!({}), "packages/access/src/provider.ts", "export class AccessProvider {}\nexport function accessProvider() {}\n");
}

/// Q15: where trimming gives a name the module already binds, the message asks
/// for a distinguishing word instead of the prefix.
#[test]
fn a_trim_that_collides_asks_for_a_distinguishing_word() {
    let code = "import { session } from '#auth/core';\nconst authSession = session;\n";

    assert_eq!(
        messages(json!({}), "identity/auth/src/cookies.ts", code),
        ["\"authSession\" repeats \"auth\" from its path, and this module already declares \"session\"; keep a word that tells them apart instead of \"auth\""]
    );
}

/// Q15: where two declarations trim to the same name, both messages ask for a
/// distinguishing word, since following both would declare the name twice.
#[test]
fn two_trims_to_one_name_ask_for_a_distinguishing_word() {
    let code = "const authSession = 1;\nconst signInSession = 2;\n";

    assert_eq!(
        messages(json!({}), "identity/auth/src/sign-in/x.ts", code),
        [
            "\"authSession\" repeats \"auth\" from its path, and another declaration here also trims to \"session\"; keep a word that tells them apart instead of \"auth\"",
            "\"signInSession\" repeats \"sign-in\" from its path, and another declaration here also trims to \"session\"; keep a word that tells them apart instead of \"sign-in\""
        ]
    );
}

/// A name declared more than once, as a `const` and `type` pair or as
/// overloads, is one entity: it gets one finding and its suggestion.
#[test]
fn a_name_declared_twice_is_judged_once() {
    let pair = "export const AssistantFrameErr = 1;\nexport type AssistantFrameErr = number;\n";
    let overloads = "\
function authParse(value: string): string;
function authParse(value: number): number;
function authParse(value: unknown): unknown { return value; }
";

    assert_eq!(
        messages(json!({}), "packages/assistant/src/contracts/frame.ts", pair),
        ["\"AssistantFrameErr\" repeats \"assistant\" from its workspace name; name it \"FrameErr\""]
    );
    assert_eq!(messages(json!({}), "identity/auth/src/parse.ts", overloads), ["\"authParse\" repeats \"auth\" from its path; name it \"parse\""]);
}

/// Q26: configured terms are not cut apart, configured folder names are never
/// reported, and exempt paths are skipped. Nothing is exempt by default.
#[test]
fn exemptions_come_from_configuration() {
    assert_eq!(
        messages(json!({}), "packages/cloudflare/src/cloudflare-access-jwt.ts", ""),
        ["file name cloudflare-access-jwt.ts repeats \"cloudflare\" from its path; name it \"access-jwt.ts\""]
    );
    assert_clean(json!({ "terms": ["cloudflare-access"] }), "packages/cloudflare/src/cloudflare-access-jwt.ts", "");

    // A term that is the whole prefix does not protect it.
    assert_eq!(messages(json!({ "terms": ["twilio"] }), "packages/twilio/src/twilio-client.ts", "").len(), 1);

    assert_eq!(messages(json!({}), "packages/runtime/runtime-tests/smoke.ts", "").len(), 1);
    assert_clean(json!({ "folders": ["runtime-tests"] }), "packages/runtime/runtime-tests/smoke.ts", "");

    assert_clean(json!({ "exempt": ["packages/knock/workflows/**"] }), "packages/knock/workflows/knock-welcome.ts", "");
}

#[test]
fn declaration_files_and_index_barrels_are_left_alone() {
    assert_clean(json!({}), "packages/twilio/src/twilio-env.d.ts", "");
    assert_clean(json!({}), "packages/twilio/src/index.ts", "export { TwilioLookupDriver } from '#twilio/lookup-driver';\n");
}

#[test]
fn options_are_validated() {
    let build = |options: Value| {
        let config = Lint { bundled: false, rules: [(RULE.to_owned(), json!(["error", options]))].into(), ignore: Vec::new() };

        Linter::new(&config).err().map(|error| error.to_string())
    };

    assert!(build(json!({ "workspaces": ["packages/*"] })).is_none());
    assert!(build(json!({})).is_some_and(|error| error.contains("workspaces is required")));
    assert!(build(json!({ "workspaces": [] })).is_some_and(|error| error.contains("workspaces is required")));
    assert!(build(json!({ "workspaces": ["packages/*"], "terms": [" "] })).is_some_and(|error| error.contains("non-blank")));
    assert!(build(json!({ "workspaces": ["packages/*"], "allow": [] })).is_some_and(|error| error.contains("unknown option")));
}

/// Acronyms, digits, a leading underscore and screaming case split into words
/// and keep their casing in the suggestion.
#[test]
fn suggestions_keep_the_name_casing() {
    let code = "const HTTPRetryLimit = 1;\nconst httpURLParser = 2;\nconst _httpCache = 3;\nconst http2Pool = 4;\n";

    assert_eq!(
        messages(json!({}), "packages/http/src/limits.ts", code),
        [
            "\"HTTPRetryLimit\" repeats \"http\" from its path; name it \"RetryLimit\"",
            "\"httpURLParser\" repeats \"http\" from its path; name it \"urlParser\"",
            "\"_httpCache\" repeats \"http\" from its path; name it \"_cache\"",
        ]
    );
}
