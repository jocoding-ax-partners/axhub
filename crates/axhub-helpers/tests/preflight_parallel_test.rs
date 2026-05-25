//! Phase 3 — `run_preflight_with_runner` parallel orchestration.
//!
//! Spec: `.plan/deploy-time-reduction/phase-3-client-cascade-reduced.md` §2.
//!
//! Three coverage angles:
//!   1. Happy path — all four probes (version + auth + cache + manifest) run
//!      and produce a merged PreflightOutput with auth_ok=true.
//!   2. CLI absent — version probe returns empty stdout; auth raw status is
//!      ignored (overridden to `cli_unavailable`).
//!   3. Sequential fallback — `AXHUB_PREFLIGHT_PARALLEL=0` env exercises the
//!      non-scoped path and produces the same output shape.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::Duration;

use axhub_helpers::preflight::{run_preflight_with_runner, SpawnResult, EXIT_OK};

fn parallel_env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn lock_parallel_env() -> MutexGuard<'static, ()> {
    parallel_env_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

fn ok(stdout: &str) -> SpawnResult {
    SpawnResult {
        exit_code: 0,
        stdout: stdout.to_string(),
        stderr: String::new(),
    }
}

fn auth_ok_stdout() -> &'static str {
    r#"{"user_email":"dev@jocodingax.ai","user_id":1,"expires_at":"2099-01-01T00:00:00Z","scopes":["deploy:write"]}"#
}

fn happy_runner(cmd: &[&str]) -> SpawnResult {
    if cmd.contains(&"--version") {
        ok("axhub 0.15.3\n")
    } else if cmd.contains(&"auth") && cmd.contains(&"status") {
        ok(auth_ok_stdout())
    } else {
        ok("[]")
    }
}

fn cli_absent_runner(cmd: &[&str]) -> SpawnResult {
    if cmd.contains(&"--version") {
        SpawnResult {
            exit_code: 127,
            stdout: String::new(),
            stderr: "axhub: not found".into(),
        }
    } else if cmd.contains(&"auth") && cmd.contains(&"status") {
        // Even if a stale auth handler answered, the merged output should
        // override to "cli_unavailable" because cli_present is false.
        ok(auth_ok_stdout())
    } else {
        ok("[]")
    }
}

#[test]
fn parallel_happy_path_merges_all_four_probes() {
    let _guard = lock_parallel_env();
    std::env::remove_var("AXHUB_PREFLIGHT_PARALLEL");
    let run = run_preflight_with_runner(happy_runner);
    assert_eq!(run.exit_code, EXIT_OK);
    assert!(run.output.cli_present);
    assert!(run.output.in_range);
    assert!(run.output.auth_ok);
    assert_eq!(run.output.user_email.as_deref(), Some("dev@jocodingax.ai"));
}

#[test]
fn parallel_cli_absent_overrides_auth_to_cli_not_found() {
    let _guard = lock_parallel_env();
    std::env::remove_var("AXHUB_PREFLIGHT_PARALLEL");
    let run = run_preflight_with_runner(cli_absent_runner);
    // exit code reflects the cli_present=false / in_range=false branch.
    assert!(!run.output.cli_present);
    assert!(!run.output.auth_ok);
    // v0.9.5: cli_absent_runner emits exit 127 + "axhub: not found" stderr,
    // which the new diagnose_cli_state() classifies as cli_not_found (more
    // specific than the legacy blanket cli_unavailable). SKILL wrappers route
    // this to /axhub:install-cli instead of mixing with config drift cases.
    assert_eq!(run.output.auth_error_code.as_deref(), Some("cli_not_found"));
}

#[test]
fn sequential_fallback_when_axhub_preflight_parallel_is_zero() {
    let _guard = lock_parallel_env();
    let prev = std::env::var("AXHUB_PREFLIGHT_PARALLEL").ok();
    std::env::set_var("AXHUB_PREFLIGHT_PARALLEL", "0");
    let run = run_preflight_with_runner(happy_runner);
    assert_eq!(run.exit_code, EXIT_OK);
    assert!(run.output.auth_ok);
    match prev {
        Some(v) => std::env::set_var("AXHUB_PREFLIGHT_PARALLEL", v),
        None => std::env::remove_var("AXHUB_PREFLIGHT_PARALLEL"),
    }
}

#[test]
fn parallel_walltime_is_bounded_by_slowest_cli_probe_not_sum() {
    let _guard = lock_parallel_env();
    std::env::remove_var("AXHUB_PREFLIGHT_PARALLEL");

    let active_cli_probes = AtomicUsize::new(0);
    let max_active_cli_probes = AtomicUsize::new(0);
    let run = run_preflight_with_runner(|cmd| {
        if cmd.contains(&"--version") || (cmd.contains(&"auth") && cmd.contains(&"status")) {
            let active = active_cli_probes.fetch_add(1, Ordering::SeqCst) + 1;
            max_active_cli_probes.fetch_max(active, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(80));
            active_cli_probes.fetch_sub(1, Ordering::SeqCst);
            if cmd.contains(&"--version") {
                ok("axhub 0.15.3\n")
            } else {
                ok(auth_ok_stdout())
            }
        } else {
            ok("[]")
        }
    });

    assert_eq!(run.exit_code, EXIT_OK);
    assert!(run.output.auth_ok);
    assert_eq!(
        max_active_cli_probes.load(Ordering::SeqCst),
        2,
        "version + auth probes should overlap instead of running sequentially"
    );
}
