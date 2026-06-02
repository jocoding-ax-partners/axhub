//! spec 006 (AC 9) — lazy auth bootstrap **succeeds** on explicit invocation in a
//! non-marker project.
//!
//! ## What "lazy auth bootstrap" is, concretely
//!
//! AC 7 made `session-start` skip the eager infra chain (token-init / warmup /
//! quality-context) in repos with **no** `axhub.yaml` marker, so a non-axhub
//! project gets a zero-footprint session — crucially, the helper auth/delegation
//! token-file (`~/.config/axhub-plugin/token`) is NOT pre-created. The spec moves
//! auth resolution "eager session-start 에서 호출 시점으로" (spec 006 §73): the
//! deploy path resolves auth **at call time** instead.
//!
//! That call-time resolution is the [`run_preflight_with_runner`] auth probe —
//! the very function `deploy-prep` invokes (`Cmd_deploy_prep → run_preflight…`).
//! It spawns `axhub auth status --json` lazily when the user actually deploys.
//! So "lazy auth bootstrap succeeds" means, in code: **for an explicit invocation
//! in a non-marker repo, the gate routes to axhub even though no token-file exists
//! (`authed=false`), and the call-time preflight auth probe then resolves the
//! authed CLI to `auth_ok` / `EXIT_OK`.**
//!
//! ## What this file pins — and what it deliberately defers
//!
//! AC 9 spans two endpoints plus a connecting branch. This file pins the two
//! **endpoints**, each made non-vacuous by a discriminator; it does NOT (and
//! cannot, without collision) pin the connecting branch, which is owned by a
//! different exit condition:
//!
//!  1. non-marker walk-up → [`MarkerStatus::Absent`] (precondition);
//!  2. **endpoint A — gate reaches the deploy path:** explicit invocation (slash
//!     `/deploy` AND the `"axhub"` keyword) → [`RoutingDecision::Axhub`] **at
//!     `authed=false`** — the zero-footprint no-token-file state does NOT
//!     short-circuit explicit intent. Discriminated by the AC 1 sibling
//!     (`routing_non_marker_implicit.rs`): bare NL in the same repo → `Ignore`;
//!  3. **endpoint B — call-time lazy auth succeeds:** the probe with an authed
//!     CLI → `auth_ok` + `EXIT_OK` (the lazy bootstrap **succeeds**);
//!  4. **discriminator for B:** the same probe with an UNAUTHORIZED CLI →
//!     `!auth_ok` + `EXIT_AUTH` (65). This makes endpoint B non-vacuous and marks
//!     the boundary to AC 10 (lazy auth failure must be an actionable stop — exit
//!     65 routes the deploy SKILL to Step 6 `axhub auth login` — never a silent
//!     pass-through).
//!
//! **Deferred — not tested here:** the connecting branch "proceed to the
//! preflight probe *only when* `decision == Axhub`" lives in the deploy preflight
//! Step 0 (`skills/deploy/SKILL.md` + `preflight.rs`), owned by the
//! `preflight-integration-complete` exit condition. `decide(...)` and
//! `run_preflight_with_runner(...)` are called side by side below with no data
//! flow between them, so a regression in that branch would NOT redden this file —
//! by design, to avoid colliding with that in-flight task. Endpoint A's
//! `Axhub` decision is the input that branch consumes; endpoint B is what it
//! gates into.
//!
//! Secondary corroboration ([`helper_token_file_lazy_bootstrap_writes_into_zero_footprint_config`]):
//! the helper-token side-effect that AC 7 stopped doing eagerly in non-marker
//! repos (`token-init`) still succeeds **on demand** from a valid source. It
//! corroborates the *helper-token* lazy mechanism only — nothing in the explicit
//! deploy path calls `token-init` lazily (deploy auth = the CLI probe of
//! endpoint B), so it is kept strictly subordinate and is NOT evidence of the
//! deploy wire.

use std::process::Command;

use axhub_helpers::preflight::{run_preflight_with_runner, SpawnResult, EXIT_AUTH, EXIT_OK};
use axhub_helpers::routing::{
    axhub_keyword_present, decide, find_marker_from, is_slash_invocation, MarkerStatus,
    RoutingDecision,
};

/// The exact explicit-keyword prompt from the AC ("deploy to axhub", Korean).
const EXPLICIT_KEYWORD_PROMPT: &str = "axhub에 배포해";
/// A representative slash invocation; the deploy preflight detects the slash from
/// its invocation context and feeds `explicit_invocation = true` into `decide`.
const EXPLICIT_SLASH_PROMPT: &str = "/deploy paydrop";

// --- shared preflight mock runners (mirrors preflight_parallel_test.rs) ---------

fn ok(stdout: &str) -> SpawnResult {
    SpawnResult {
        exit_code: 0,
        stdout: stdout.to_string(),
        stderr: String::new(),
    }
}

/// CLI present, in-range version, and an authenticated `auth status` — the state
/// a successfully logged-in user presents at deploy time.
fn authed_cli_runner(cmd: &[&str]) -> SpawnResult {
    if cmd.contains(&"--version") {
        ok("axhub 0.15.3\n")
    } else if cmd.contains(&"auth") && cmd.contains(&"status") {
        ok(r#"{"user_email":"dev@jocodingax.ai","user_id":1,"expires_at":"2099-01-01T00:00:00Z","scopes":["deploy:write"]}"#)
    } else {
        ok("[]")
    }
}

/// CLI present and in range, but `auth status` reports UNAUTHORIZED. Lazy auth
/// must fail *closed* here (exit 65), never silently pass through.
fn unauthed_cli_runner(cmd: &[&str]) -> SpawnResult {
    if cmd.contains(&"--version") {
        ok("axhub 0.15.3\n")
    } else if cmd.contains(&"auth") && cmd.contains(&"status") {
        ok(r#"{"code":"unauthorized","detail":"no active session"}"#)
    } else {
        ok("[]")
    }
}

/// Build a non-marker git repo: a `.git` directory so the walk-up terminates at
/// the repo root, and deliberately NO `axhub.yaml`. Mirrors the AC 1 fixture.
fn non_marker_repo() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("create tempdir");
    std::fs::create_dir(tmp.path().join(".git")).expect("mkdir .git");
    tmp
}

// --- step 1: precondition -------------------------------------------------------

/// A `.git` repo without `axhub.yaml` resolves to a confirmed `Absent` (not the
/// `Unknown` error fallback). Establishes the non-marker half independently so the
/// routing assertions below rest on a verified input.
#[test]
fn non_marker_repo_walk_up_is_absent() {
    let repo = non_marker_repo();
    let nested = repo.path().join("apps").join("web");
    std::fs::create_dir_all(&nested).expect("mkdir nested");
    assert_eq!(
        find_marker_from(&nested),
        MarkerStatus::Absent,
        "a .git repo without axhub.yaml must be Absent, not Unknown"
    );
}

// --- step 2: explicit invocation reaches the deploy path despite no marker/auth -

/// Slash invocation (rule 0) → `Axhub` regardless of marker/auth. The load-bearing
/// row is `authed = false`: in a zero-footprint non-marker repo no token-file
/// exists, yet the gate must still route to axhub so the deploy path — and its
/// call-time lazy auth — is reached.
#[test]
fn explicit_slash_invocation_reaches_deploy_path_in_non_marker() {
    let repo = non_marker_repo();
    let marker = find_marker_from(repo.path());
    assert_eq!(marker, MarkerStatus::Absent, "precondition: no marker");
    assert!(
        is_slash_invocation(EXPLICIT_SLASH_PROMPT),
        "the deploy preflight must read \"{EXPLICIT_SLASH_PROMPT}\" as a slash invocation"
    );

    for authed in [false, true] {
        let decision = decide(
            EXPLICIT_SLASH_PROMPT,
            marker,
            authed,
            /* explicit_invocation = */ true,
        );
        assert_eq!(
            decision,
            RoutingDecision::Axhub,
            "slash /deploy in a non-marker repo must route to axhub (authed={authed})"
        );
    }
}

/// Explicit `"axhub"` keyword (rule b) → `Axhub`, marker-independent, even at
/// `authed = false`. This is the AC's literal "axhub에 배포해" prompt.
#[test]
fn explicit_axhub_keyword_reaches_deploy_path_in_non_marker() {
    let repo = non_marker_repo();
    let marker = find_marker_from(repo.path());
    assert_eq!(marker, MarkerStatus::Absent, "precondition: no marker");
    assert!(
        axhub_keyword_present(EXPLICIT_KEYWORD_PROMPT),
        "\"{EXPLICIT_KEYWORD_PROMPT}\" must be read as an explicit axhub keyword"
    );

    for authed in [false, true] {
        let decision = decide(
            EXPLICIT_KEYWORD_PROMPT,
            marker,
            authed,
            /* explicit_invocation = */ false,
        );
        assert_eq!(
            decision,
            RoutingDecision::Axhub,
            "\"axhub에 배포해\" in a non-marker repo must route to axhub (authed={authed})"
        );
    }
}

// --- step 3: the call-time lazy auth probe succeeds for an authed CLI ----------

/// The deploy path's call-time auth resolution (`deploy-prep` →
/// `run_preflight_with_runner`). With an authenticated CLI it yields `auth_ok` and
/// `EXIT_OK` — the lazy bootstrap succeeds at call time, with no eager
/// session-start token-init required.
#[test]
fn call_time_auth_probe_succeeds_for_authed_cli() {
    let run = run_preflight_with_runner(authed_cli_runner);
    assert!(run.output.cli_present, "authed CLI must be present");
    assert!(run.output.in_range, "version must be in supported range");
    assert!(run.output.auth_ok, "authed CLI must resolve to auth_ok");
    assert_eq!(run.output.user_email.as_deref(), Some("dev@jocodingax.ai"));
    assert_eq!(
        run.exit_code, EXIT_OK,
        "successful lazy auth must exit OK so the deploy proceeds"
    );
}

// --- step 4: discriminator — lazy auth FAILS CLOSED (non-vacuity + AC 10 edge) --

/// The same call-time probe with an UNAUTHORIZED CLI must fail closed: `!auth_ok`
/// and `EXIT_AUTH` (65). Without this, step 3's success would be vacuous. Exit 65
/// is what routes the deploy SKILL to Step 6 (`axhub auth login`) — the actionable
/// stop, never a silent pass-through (boundary to AC 10).
#[test]
fn call_time_auth_probe_fails_closed_for_unauthed_cli() {
    let run = run_preflight_with_runner(unauthed_cli_runner);
    assert!(run.output.cli_present, "CLI is present, only auth is missing");
    assert!(!run.output.auth_ok, "UNAUTHORIZED CLI must not read as auth_ok");
    assert_eq!(
        run.output.auth_error_code.as_deref(),
        Some("unauthorized"),
        "the failure must carry an actionable auth_error_code, not a blank pass-through"
    );
    assert_eq!(
        run.exit_code, EXIT_AUTH,
        "lazy auth failure must exit 65 (routes to Step 6 axhub auth login)"
    );
}

// --- headline: both AC 9 endpoints co-located ----------------------------------

/// Both AC 9 endpoints in one place so a break in EITHER endpoint goes red:
/// explicit invocation (both modalities) in a non-marker repo routes to `Axhub`
/// even with no token-file (`authed=false`), and the call-time auth probe with an
/// authed CLI resolves to `EXIT_OK`. Together: "lazy auth bootstrap succeeds on
/// explicit invocation in a non-marker project".
///
/// NOTE: the two `decide(...)` calls and the `run_preflight_with_runner(...)` call
/// are intentionally NOT data-wired — the "route to preflight iff decision==Axhub"
/// branch is the deploy preflight Step 0, owned by `preflight-integration-complete`
/// (see module doc). This test pins the endpoints that branch connects, not the
/// branch itself.
#[test]
fn lazy_bootstrap_endpoints_explicit_non_marker() {
    let repo = non_marker_repo();
    let marker = find_marker_from(repo.path());
    assert_eq!(marker, MarkerStatus::Absent, "precondition: no marker");

    // Both explicit modalities reach the deploy path despite zero footprint.
    let via_slash = decide(EXPLICIT_SLASH_PROMPT, marker, /* authed */ false, true);
    let via_keyword = decide(EXPLICIT_KEYWORD_PROMPT, marker, /* authed */ false, false);
    assert_eq!(via_slash, RoutingDecision::Axhub, "slash must reach deploy");
    assert_eq!(
        via_keyword,
        RoutingDecision::Axhub,
        "axhub keyword must reach deploy"
    );

    // At the reached deploy path, the lazy call-time auth bootstrap succeeds.
    let run = run_preflight_with_runner(authed_cli_runner);
    assert!(
        run.output.auth_ok && run.exit_code == EXIT_OK,
        "explicit non-marker deploy must resolve auth lazily and proceed"
    );
}

// --- secondary corroboration: helper-token lazy bootstrap (token-init) ----------

/// The helper auth token-file side-effect that AC 7 stopped running eagerly in
/// non-marker repos still succeeds **on demand**. Spawning `axhub-helpers
/// token-init` against a fresh (zero-footprint) `XDG_CONFIG_HOME` with a valid
/// `AXHUB_TOKEN` source writes the token-file that was absent before — call-time
/// bootstrap, no eager session-start. Subordinate to the preflight composition
/// above; included to pin the second lazy mechanism the spec names.
#[test]
fn helper_token_file_lazy_bootstrap_writes_into_zero_footprint_config() {
    let work = tempfile::tempdir().expect("create tempdir");
    let cfg = work.path().join("cfg");
    let token_path = cfg.join("axhub-plugin").join("token");

    // Zero-footprint precondition: nothing under the fresh config dir yet.
    assert!(
        !token_path.exists(),
        "precondition: a non-marker session leaves no helper token-file"
    );

    let output = Command::new(env!("CARGO_BIN_EXE_axhub-helpers"))
        .arg("token-init")
        .arg("--json")
        // env:AXHUB_TOKEN is checked before any CLI spawn, so this needs no live
        // OAuth. Token must be >=16 chars with no whitespace/control bytes.
        .env("AXHUB_TOKEN", "axhubtok_0123456789abcdef")
        .env("XDG_CONFIG_HOME", &cfg)
        .env("HOME", work.path().join("home"))
        .output()
        .expect("spawn axhub-helpers token-init");

    assert!(
        output.status.success(),
        "lazy token-init must exit 0 (stdout={}, stderr={})",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"stored\":true"),
        "token-init must report a stored token: {stdout}"
    );
    assert!(
        token_path.exists(),
        "lazy bootstrap must create the helper token-file that was absent before"
    );
}
