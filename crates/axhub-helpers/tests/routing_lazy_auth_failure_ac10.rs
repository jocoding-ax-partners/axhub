//! spec 006 (AC 10) — a **lazy auth failure** on the explicit axhub path produces an
//! actionable error, never a silent pass-through.
//!
//! ## Where this sits relative to AC 9
//!
//! AC 9 (`routing_lazy_auth_explicit_ac9.rs`) owns the *success* endpoint: an
//! explicit invocation in a non-marker repo reaches the deploy path with no
//! token-file (`authed=false`), and the call-time auth probe with an authed CLI
//! resolves to `auth_ok` + `EXIT_OK`. Its `unauthed` test exists only as a
//! *discriminator* (so the success is non-vacuous) and its module doc explicitly
//! defers the full failure contract to AC 10:
//!
//! > "exit 65 routes the deploy SKILL to Step 6 `axhub auth login` — never a
//! >  silent pass-through (boundary to AC 10)."
//!
//! This file **owns** that boundary. It deliberately does NOT re-litigate "unauthed
//! CLI → exit 65 + code" as a headline (AC 9 + `preflight.rs`'s own unit tests
//! already pin that single row). Instead it closes three things AC 9 leaves open:
//!
//!  1. **Auth-layer contract (additive over AC 9):** *every* auth-failure mode
//!     (logged-out / expired / malformed / missing-fields), not just the one
//!     `"unauthorized"` shape, with a present CLI in range, resolves to
//!     `EXIT_AUTH` (65) carrying a **non-empty** `auth_error_code` — and **never**
//!     `EXIT_OK`. "Never EXIT_OK on an auth failure" is the machine form of "never
//!     continue-as-if-authed / never silently pass through".
//!  2. **The actionable-message linkage (the half AC 9 names but doesn't test):**
//!     exit 65 → [`catalog::classify`] renders a concrete *re-login* recovery
//!     entry (emotion + cause + action all naming `로그인` / "다시 로그인"), NOT the
//!     `default_entry` "알 수 없는 에러" fallback and NOT the `EXIT_OK` celebration.
//!     This is tested against the **real helper renderer** (`classify-exit`'s
//!     source of truth — `catalog::classify`, fed from `data/catalog.json`), not by
//!     grepping SKILL.md prose, so it proves behavior rather than a string's
//!     existence.
//!  3. **The explicit↔implicit asymmetry (the polarity that makes AC 10
//!     non-vacuous):** a *silent* `Ignore` pass-through is the *correct* outcome
//!     for an implicit bare-NL deploy in an unauthed non-marker repo (zero
//!     footprint, spec §99 / AC 18). The whole point of AC 10 is that the **explicit**
//!     path must NOT be downgraded to that silent `Ignore` when auth fails — it
//!     surfaces the typed exit-65 stop instead. Both are co-located so the
//!     asymmetry is explicit and a regression collapsing one into the other reddens.
//!
//! ## What is deliberately NOT wired
//!
//! As in AC 9, the routing `decide(...)` calls and the `run_preflight_with_runner(...)`
//! calls are not data-threaded — the "proceed to the preflight probe iff
//! decision==Axhub" branch is the deploy preflight Step 0, owned by
//! `preflight-integration-complete`. This file pins the endpoints that branch
//! connects (explicit → reaches deploy; auth-fail → actionable exit-65 stop +
//! rendered guidance), not the connecting branch.

use axhub_helpers::catalog::{classify, ErrorEntry};
use axhub_helpers::preflight::{
    run_preflight_with_runner, SpawnResult, EXIT_AUTH, EXIT_OK, EXIT_USAGE,
};
use axhub_helpers::routing::{
    axhub_keyword_present, decide, find_marker_from, is_slash_invocation, MarkerStatus,
    RoutingDecision,
};

// --- mock preflight runners (mirrors AC 9 / preflight_parallel_test.rs) ----------

fn ok(stdout: &str) -> SpawnResult {
    SpawnResult {
        exit_code: 0,
        stdout: stdout.to_string(),
        stderr: String::new(),
    }
}

/// A CLI that is present and in supported range, whose `auth status --json`
/// returns `auth_json`. Only the auth surface varies, so the resulting exit code
/// is driven purely by the auth state (isolates the auth-failure contract).
fn cli_with_auth(auth_json: &'static str) -> impl Fn(&[&str]) -> SpawnResult {
    move |cmd: &[&str]| {
        if cmd.contains(&"--version") {
            ok("axhub 0.15.3\n")
        } else if cmd.contains(&"auth") && cmd.contains(&"status") {
            ok(auth_json)
        } else {
            ok("[]")
        }
    }
}

/// Build a non-marker git repo (`.git`, deliberately no `axhub.yaml`) so the
/// walk-up terminates at `Absent`. Mirrors the AC 1 / AC 9 fixture.
fn non_marker_repo() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("create tempdir");
    std::fs::create_dir(tmp.path().join(".git")).expect("mkdir .git");
    tmp
}

/// The explicit-keyword and slash prompts from AC 9 — the two explicit modalities.
const EXPLICIT_KEYWORD_PROMPT: &str = "axhub에 배포해";
const EXPLICIT_SLASH_PROMPT: &str = "/deploy paydrop";

/// Every distinct auth-failure mode `parse_auth_status` can land in, with the
/// `auth_error_code` it must surface. Logged-out, expired, malformed transport,
/// and a valid-JSON-but-wrong-shape response are materially different failures;
/// all four must be actionable (exit 65 + non-empty code), none may pass through.
const AUTH_FAILURE_MODES: &[(&str, &str)] = &[
    // logged out / no active session
    (
        r#"{"code":"unauthorized","detail":"no active session"}"#,
        "unauthorized",
    ),
    // token expired — the canonical "로그인이 만료됐어요" Step 6 case
    (
        r#"{"code":"token_expired","detail":"session expired"}"#,
        "token_expired",
    ),
    // malformed / non-JSON transport (CLI emitted garbage, e.g. a stray banner)
    ("not-json-at-all", "parse_error"),
    // valid JSON but missing both `code` and `user_email` (unexpected shape)
    (r#"{"unexpected":"shape"}"#, "unknown_shape"),
];

// --- precondition (light — already fully pinned by AC 5 / AC 9) ------------------

/// The explicit path reaches the deploy probe in a zero-footprint non-marker repo
/// even at `authed=false`. Kept to ONE row: this is the input AC 10's contract
/// applies to, not AC 10's headline (AC 5/AC 9 own "explicit reaches deploy").
#[test]
fn explicit_invocation_reaches_deploy_in_non_marker() {
    let repo = non_marker_repo();
    let marker = find_marker_from(repo.path());
    assert_eq!(marker, MarkerStatus::Absent, "precondition: no marker");
    assert!(is_slash_invocation(EXPLICIT_SLASH_PROMPT));
    assert!(axhub_keyword_present(EXPLICIT_KEYWORD_PROMPT));

    // Both explicit modalities route to axhub with no token-file, so the deploy
    // path — and its call-time lazy auth — is actually reached (not short-circuited).
    assert_eq!(
        decide(EXPLICIT_SLASH_PROMPT, marker, /* authed */ false, true),
        RoutingDecision::Axhub,
    );
    assert_eq!(
        decide(
            EXPLICIT_KEYWORD_PROMPT,
            marker,
            /* authed */ false,
            false
        ),
        RoutingDecision::Axhub,
    );
}

// --- contract 1: auth-layer — every failure mode is an actionable stop ----------

/// **Additive over AC 9.** AC 9 pins only the `"unauthorized"` row. Here every
/// auth-failure mode, with the CLI present and in range, must resolve to:
/// `!auth_ok`, `exit_code == EXIT_AUTH (65)`, and a **non-empty** `auth_error_code`.
/// Crucially it must **never** be `EXIT_OK` — that would be the silent
/// "continue-as-if-authed" pass-through the AC forbids.
#[test]
fn every_auth_failure_mode_exits_65_with_actionable_code_never_ok() {
    for (auth_json, expected_code) in AUTH_FAILURE_MODES {
        let run = run_preflight_with_runner(cli_with_auth(auth_json));

        assert!(
            run.output.cli_present,
            "CLI is present for mode {expected_code:?}; only auth is failing"
        );
        assert!(
            run.output.in_range,
            "version must be in range so the exit code reflects AUTH, not USAGE ({expected_code:?})"
        );
        assert!(
            !run.output.auth_ok,
            "auth-failure mode {expected_code:?} must not read as authed"
        );
        let code = run
            .output
            .auth_error_code
            .as_deref()
            .expect("auth failure must carry an auth_error_code, not a blank pass-through");
        assert!(
            !code.is_empty(),
            "auth_error_code must be non-empty for {expected_code:?} (actionable, not blank)"
        );
        assert_eq!(
            code, *expected_code,
            "auth_error_code must name the specific failure mode so the SKILL routes precisely"
        );
        assert_eq!(
            run.exit_code, EXIT_AUTH,
            "auth failure {expected_code:?} must exit 65 (routes to Step 6 auth login)"
        );
        assert_ne!(
            run.exit_code, EXIT_OK,
            "auth failure {expected_code:?} must NEVER resolve to EXIT_OK — that is the \
             silent continue-as-if-authed pass-through AC 10 forbids"
        );
    }
}

// --- contract 2: linkage — exit 65 renders actionable re-login guidance ---------

/// The half AC 9 *names* but never tests: exit 65 must map to an **actionable**
/// recovery message via the real helper renderer (`catalog::classify`, the source
/// of truth behind `classify-exit` and the generated empathy catalog). The entry
/// must be the concrete *re-login* guidance — naming `로그인` / "다시 로그인" across
/// emotion + cause + action — and must NOT be the `default_entry` "알 수 없는 에러"
/// fallback (a blank, non-actionable stop is itself a kind of silent failure).
#[test]
fn exit_65_renders_actionable_auth_login_guidance() {
    let entry: ErrorEntry = classify(EXIT_AUTH, "");

    // Not the unknown-error default fallback — the guidance is specific & actionable.
    assert!(
        !entry.cause.contains("알 수 없는 에러"),
        "exit 65 must have a dedicated catalog entry, not the unknown-error fallback"
    );

    // The recovery is re-login: every visible field steers the user to log in again.
    assert!(
        entry.emotion.contains("로그인"),
        "exit-65 emotion must frame this as a login/auth issue: {:?}",
        entry.emotion
    );
    assert!(
        entry.cause.contains("로그인"),
        "exit-65 cause must explain the auth/login expiry: {:?}",
        entry.cause
    );
    assert!(
        entry.action.contains("로그인"),
        "exit-65 action must guide the user to log in again (axhub auth login): {:?}",
        entry.action
    );
    // An actionable stop offers a recovery button, not a dead end.
    let button = entry
        .button
        .as_deref()
        .expect("exit-65 entry must offer recovery buttons (actionable, not a dead end)");
    assert!(
        button.contains("다시 로그인"),
        "exit-65 button must include a '다시 로그인' (re-login) recovery action: {button:?}"
    );
}

/// The rendered guidance is genuinely *tied to the failure exit code*, not a
/// constant: classify(65) must differ from the EXIT_OK success/celebration entry.
/// If an auth failure ever rendered the success message, that is a silent
/// pass-through dressed up as success.
#[test]
fn auth_failure_message_is_not_the_success_message() {
    let auth_fail = classify(EXIT_AUTH, "");
    let success = classify(EXIT_OK, "");
    assert_ne!(
        auth_fail, success,
        "auth-failure guidance must not be the EXIT_OK success message"
    );
    assert!(
        success.emotion.contains("축하해요") && !auth_fail.emotion.contains("축하해요"),
        "an auth failure must never render the deploy-success celebration"
    );
}

// --- contract 3: the explicit↔implicit asymmetry (polarity discriminator) -------

/// THE discriminator that makes AC 10 non-vacuous. A *silent* `Ignore`
/// pass-through is the **correct** outcome for an implicit bare-NL deploy by an
/// unauthed user in a non-marker repo (zero footprint, spec §99 / AC 18). AC 10's
/// claim is that the **explicit** path is NOT downgraded to that silent `Ignore`
/// when auth fails: it routes to axhub (reaching the deploy probe) and the probe
/// surfaces the typed exit-65 stop. Co-located so the asymmetry — silent for
/// implicit, actionable for explicit — is locked against either side collapsing
/// into the other.
#[test]
fn explicit_auth_fail_surfaces_while_implicit_unauthed_stays_silent() {
    let repo = non_marker_repo();
    let marker = find_marker_from(repo.path());
    assert_eq!(marker, MarkerStatus::Absent, "precondition: no marker");

    // Implicit bare-NL deploy, unauthed, no marker → the LEGITIMATE silent
    // pass-through. No axhub footprint, no actionable stop — and that's correct.
    let implicit = decide("배포해", marker, /* authed */ false, false);
    assert_eq!(
        implicit,
        RoutingDecision::Ignore,
        "implicit bare-NL + unauthed + non-marker is the correct silent pass-through"
    );

    // Explicit deploy, unauthed, no marker → routes to axhub (NOT silently
    // Ignored), so the deploy path is reached...
    let explicit = decide(EXPLICIT_SLASH_PROMPT, marker, /* authed */ false, true);
    assert_eq!(
        explicit,
        RoutingDecision::Axhub,
        "explicit invocation must NOT be downgraded to the implicit silent Ignore"
    );
    assert_ne!(
        explicit, implicit,
        "explicit and implicit must diverge: the explicit auth path cannot collapse \
         into the implicit silent pass-through"
    );

    // ...and at that reached deploy path, the lazy auth probe fails CLOSED with an
    // actionable exit-65 stop — never the silent EXIT_OK proceed.
    let run = run_preflight_with_runner(cli_with_auth(
        r#"{"code":"unauthorized","detail":"no active session"}"#,
    ));
    assert!(!run.output.auth_ok);
    assert_eq!(
        run.exit_code, EXIT_AUTH,
        "the explicit path's auth failure must surface as an actionable exit-65 stop"
    );
}

// --- headline: lazy auth failure on the explicit path is an actionable stop -----

/// Both halves of AC 10 in one place so a break in EITHER reddens: an explicit
/// invocation in a zero-footprint non-marker repo reaches the deploy probe; the
/// lazy auth probe fails closed (exit 65 + non-empty `auth_error_code`, never
/// EXIT_OK); and exit 65 renders the actionable re-login guidance. Together:
/// "lazy auth failure produces an actionable error message, not a silent
/// pass-through."
///
/// NOTE (as in AC 9): the `decide(...)` and `run_preflight_with_runner(...)` calls
/// are intentionally not data-wired — the "route to preflight iff decision==Axhub"
/// branch is the deploy preflight Step 0 (`preflight-integration-complete`). This
/// pins the endpoints that branch connects.
#[test]
fn lazy_auth_failure_is_actionable_stop_on_explicit_non_marker() {
    let repo = non_marker_repo();
    let marker = find_marker_from(repo.path());
    assert_eq!(marker, MarkerStatus::Absent, "precondition: no marker");

    // Explicit → reaches deploy (no token-file required to get there).
    assert_eq!(
        decide(
            EXPLICIT_KEYWORD_PROMPT,
            marker,
            /* authed */ false,
            false
        ),
        RoutingDecision::Axhub,
    );

    // The reached probe fails closed for an unauthed CLI: actionable, never EXIT_OK.
    let run = run_preflight_with_runner(cli_with_auth(
        r#"{"code":"unauthorized","detail":"no active session"}"#,
    ));
    assert!(!run.output.auth_ok, "unauthed CLI must not read as authed");
    assert!(
        run.output.auth_error_code.is_some(),
        "the stop must carry an auth_error_code, not a blank pass-through"
    );
    assert_eq!(run.exit_code, EXIT_AUTH, "actionable exit-65 stop");
    assert_ne!(
        run.exit_code, EXIT_OK,
        "must never silently proceed as if authed"
    );
    // exit 65 is a CLI-present auth failure, distinct from the CLI-absent USAGE(64)
    // path — so the user is routed to auth login, not install-cli.
    assert_ne!(
        run.exit_code, EXIT_USAGE,
        "an auth failure (CLI present) must route to auth login, not the CLI-missing path"
    );

    // And that exit code renders concrete, actionable re-login guidance.
    let entry = classify(run.exit_code, "");
    assert!(
        entry.action.contains("로그인")
            && entry
                .button
                .as_deref()
                .is_some_and(|b| b.contains("로그인")),
        "exit-65 must render actionable axhub-auth-login guidance: {entry:?}"
    );
}
