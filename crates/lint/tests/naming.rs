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
    assert_eq!(
        messages(json!({}), "packages/statuses/src/status-badge.ts", ""),
        ["file name status-badge.ts repeats \"status\" from its path; name it \"badge.ts\""]
    );
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

/// Q14 and unit 5.last: names kept on purpose come from `allow`, exactly or
/// with `*`, for declarations, files and folders alike.
#[test]
fn allowed_names_are_kept() {
    let code = "export class LivingServiceProvider {}\nexport const LivingBootContext = 1;\nexport class LivingRoomRepository {}\n";
    let allow = json!({ "allow": ["*ServiceProvider", "LivingBootContext", "living-routes", "living-*-tools"] });

    assert_eq!(messages(json!({}), "packages/living/src/providers.ts", code).len(), 3);
    assert_eq!(
        messages(allow.clone(), "packages/living/src/providers.ts", code),
        ["\"LivingRoomRepository\" repeats \"living\" from its workspace name; name it \"RoomRepository\""]
    );
    assert_clean(allow.clone(), "packages/living/src/living-routes.ts", "");
    assert_clean(allow, "packages/living/src/living-chat-tools/index.ts", "");
}

/// A name with an exported declaration is judged as an export, whichever of
/// its declarations comes first.
#[test]
fn a_name_exported_anywhere_is_judged_as_an_export() {
    assert_eq!(
        messages(json!({}), "packages/twilio/src/lookup/modes.ts", "const TwilioLookupMode = 1;\nexport type TwilioLookupMode = number;\n"),
        ["\"TwilioLookupMode\" repeats \"twilio\" from its workspace name; name it \"LookupMode\""]
    );
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
        let config = Lint { bundled: false, rules: [(RULE.to_owned(), json!(["error", options]))].into(), ignore: Vec::new(), ..Default::default() };

        Linter::new(&config).err().map(|error| error.to_string())
    };

    assert!(build(json!({ "workspaces": ["packages/*"] })).is_none());
    assert!(build(json!({})).is_some_and(|error| error.contains("workspaces is required")));
    assert!(build(json!({ "workspaces": [] })).is_some_and(|error| error.contains("workspaces is required")));
    assert!(build(json!({ "workspaces": ["packages/*"], "terms": [" "] })).is_some_and(|error| error.contains("non-blank")));
    assert!(build(json!({ "workspaces": ["packages/*"], "allow": ["*ServiceProvider"] })).is_none());
    assert!(build(json!({ "workspaces": ["packages/*"], "allow": [""] })).is_some_and(|error| error.contains("non-blank")));
    assert!(build(json!({ "workspaces": ["packages/*"], "keep": [] })).is_some_and(|error| error.contains("unknown option")));
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

const WORDS: &str = "naming/max-name-words";

/// The names `naming/max-name-words` reports in `code`, linted as `file`.
fn long_names(options: Value, file: &str, code: &str) -> Vec<String> {
    let result = lint(&only(WORDS, &[options]), file, code, false);

    assert!(result.diagnostics.iter().all(|d| d.rule == WORDS), "{:?}", result.diagnostics);
    result.diagnostics.into_iter().map(|d| d.message.split(" has ").next().unwrap_or_default().to_owned()).collect()
}

/// The names from the request: two words pass, three or more are reported,
/// whatever the casing.
#[test]
fn names_over_two_words_are_reported() {
    let code = "\
export class QueueRouter {}
const correlationId = 1;
function workerEnv() {}
export class BackgroundExceptionBoundary {}
type ObservedExecutionContext = string;
export function runWithAuthBackgroundScope() {}
const readBookingProposalTtlMs = () => 1;
export const DEFAULT_AUTH_SESSION_LIFETIME_SECONDS = 60;
";

    assert_eq!(
        long_names(json!({}), "src/a.ts", code),
        [
            "\"BackgroundExceptionBoundary\"",
            "\"ObservedExecutionContext\"",
            "\"runWithAuthBackgroundScope\"",
            "\"readBookingProposalTtlMs\"",
            "\"DEFAULT_AUTH_SESSION_LIFETIME_SECONDS\"",
        ]
    );
}

#[test]
fn the_message_counts_the_words_and_points_at_an_abstraction() {
    let result = lint(&only(WORDS, &[json!({})]), "src/a.ts", "const readBookingProposalTtlMs = 1;\n", false);

    assert_eq!(
        result.diagnostics.iter().map(|d| d.message.as_str()).collect::<Vec<_>>(),
        [
            "\"readBookingProposalTtlMs\" has 5 words, more than 2; use a shorter name, or an abstraction that carries the context: a module, namespace or class whose name makes the long name unnecessary"
        ]
    );
}

/// An acronym run is one word and ends before the word it leads; digits stay
/// with their word.
#[test]
fn acronyms_and_digits_split_like_the_prefix_rule() {
    assert_clean_words(json!({}), "src/a.ts", "const HTTPServer = 1;\nconst oauth2Client = 2;\nconst userID = 3;\nconst _privateCache = 4;\n");
    assert_eq!(long_names(json!({}), "src/a.ts", "const HTTPServerPool = 1;\n"), ["\"HTTPServerPool\""]);
}

fn assert_clean_words(options: Value, file: &str, code: &str) {
    assert_eq!(long_names(options, file, code), Vec::<String>::new(), "{file}");
}

/// Every kind of declaration is judged, at any depth.
#[test]
fn every_declaration_kind_is_judged() {
    let code = "\
interface RetryPolicyShape { someLongMemberName: string }
enum RetryBackoffKind { LinearWithJitter, Fixed }
namespace RetryPolicyTools {}
class Scheduler {
    pendingJobCount = 0;
    #lastRunAt = 0;
    static accessor nextRunHint = 1;
    scheduleNextRun(retryDelayMs: number) {
        const firstFailedJob = 1;
        try {} catch (lastSeenError) {}
        for (const queuedJobId of []) {}
        const [primaryWorkerSlot, ...otherWorkerSlots] = [];
        const { slot: chosenWorkerSlot } = {};
        return function innerRetryLoop() {};
    }
}
";

    assert_eq!(
        long_names(json!({}), "src/a.ts", code),
        [
            "\"RetryPolicyShape\"",
            "\"RetryBackoffKind\"",
            "\"LinearWithJitter\"",
            "\"RetryPolicyTools\"",
            "\"pendingJobCount\"",
            "\"lastRunAt\"",
            "\"nextRunHint\"",
            "\"scheduleNextRun\"",
            "\"retryDelayMs\"",
            "\"firstFailedJob\"",
            "\"lastSeenError\"",
            "\"queuedJobId\"",
            "\"primaryWorkerSlot\"",
            "\"otherWorkerSlots\"",
            "\"chosenWorkerSlot\"",
            "\"innerRetryLoop\"",
        ]
    );
}

/// Names a module, a property or a parent type dictates are not the author's
/// to shorten.
#[test]
fn dictated_names_are_left_alone() {
    let code = "\
import { someImportedHelper } from './helpers';
import defaultExportedThing, * as wholeNamespaceImport from './things';
const { bindingNameHere, withDefaultValue = 1 } = someImportedHelper;
const config = { someObjectKey: 1, someObjectMethod() {} };
function identity<TResultValue>(value: TResultValue) { return value; }
class ConcreteSink extends Base {
    flushPendingWrites() {}
    pendingWriteCount = 0;
    constructor(private readonly writeBufferSize: number) { super(); }
}
class PolicySink implements Sink {
    flushPendingWrites() {}
}
";

    assert_clean_words(json!({}), "src/a.ts", code);
}

/// Ambient declarations describe code written elsewhere.
#[test]
fn ambient_declarations_and_declaration_files_are_left_alone() {
    let code = "\
declare const injectedBuildVersion: string;
declare function externalBridgeCall(argumentList: string[]): void;
declare class ExternalWidgetHost { mountWidgetHere(): void }
declare module 'some-module' { export const moduleLevelThing: number; }
declare global { interface WindowWithBridge {} }
declare namespace LegacyGlobalApi { const innerLegacyThing: number; }
declare enum ExternalStatusCode { NotFoundHere }
class Widget { declare injectedHostValue: number; }
";

    assert_clean_words(json!({}), "src/a.ts", code);
    assert_clean_words(json!({}), "src/env-type-defs.d.ts", "export const someDeclaredConstant: number;\n");
}

/// Overloads, merged declarations and accessor pairs are one name.
#[test]
fn a_name_declared_twice_is_reported_once() {
    let code = "\
function parseRetryWindow(value: string): string;
function parseRetryWindow(value: number): number;
function parseRetryWindow(value: unknown): unknown { return value; }
interface RetryWindowShape { a: 1 }
interface RetryWindowShape { b: 2 }
class Clock {
    get currentTickCount() { return 1; }
    set currentTickCount(value) {}
}
";

    assert_eq!(long_names(json!({}), "src/a.ts", code), ["\"parseRetryWindow\"", "\"RetryWindowShape\"", "\"currentTickCount\""]);
}

/// The file name is judged without its extensions, for scripts only.
#[test]
fn file_names_are_judged_without_extensions() {
    assert_eq!(long_names(json!({}), "src/booking-proposal-ttl.ts", ""), ["file name booking-proposal-ttl.ts"]);
    assert_eq!(long_names(json!({}), "src/queueRouterImpl.ts", ""), ["file name queueRouterImpl.ts"]);
    assert_clean_words(json!({ "ignoreTests": false }), "src/queue-router.test.ts", "");
    assert_clean_words(json!({}), "src/queue_router.mjs", "");
}

/// `max`, `allow`, `exempt` and `ignoreTests` come from the options.
#[test]
fn options_change_what_is_reported() {
    let code = "const correlationId = 1;\nconst requestCorrelationId = 2;\nexport class LivingServiceProvider {}\n";

    assert_eq!(long_names(json!({ "max": 1 }), "src/a.ts", code), ["\"correlationId\"", "\"requestCorrelationId\"", "\"LivingServiceProvider\""]);
    assert_eq!(long_names(json!({ "max": 3 }), "src/a.ts", code), Vec::<String>::new());
    assert_eq!(long_names(json!({ "allow": ["*ServiceProvider"] }), "src/a.ts", code), ["\"requestCorrelationId\""]);
    assert_clean_words(json!({ "exempt": ["src/generated/**"] }), "src/generated/a.ts", code);

    // Test files are skipped unless `ignoreTests` is false.
    assert_clean_words(json!({}), "src/a.test.ts", code);
    assert_eq!(long_names(json!({ "ignoreTests": false }), "src/a.test.ts", code).len(), 2);
}

#[test]
fn word_options_are_validated() {
    let build = |options: Value| {
        let config = Lint { bundled: false, rules: [(WORDS.to_owned(), json!(["error", options]))].into(), ignore: Vec::new(), ..Default::default() };

        Linter::new(&config).err().map(|error| error.to_string())
    };
    let bare = Lint { bundled: false, rules: [(WORDS.to_owned(), json!("error"))].into(), ignore: Vec::new(), ..Default::default() };

    assert!(Linter::new(&bare).is_ok());
    assert!(build(json!({ "max": 1, "allow": ["*Id"], "exempt": ["src/**"], "ignoreTests": false })).is_none());
    assert!(build(json!({ "max": 0 })).is_some_and(|error| error.contains("at least 1")));
    assert!(build(json!({ "allow": [""] })).is_some_and(|error| error.contains("non-blank")));
    assert!(build(json!({ "workspaces": [] })).is_some_and(|error| error.contains("unknown option")));
}
