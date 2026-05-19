use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnResult {
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub fn spawn_sync(cmd: &[&str]) -> anyhow::Result<SpawnResult> {
    anyhow::ensure!(!cmd.is_empty(), "command is empty");
    let output = Command::new(cmd[0]).args(&cmd[1..]).output()?;
    Ok(SpawnResult {
        exit_code: output.status.code(),
        signal: signal_code(&output.status),
        stdout: String::from_utf8(output.stdout)?,
        stderr: String::from_utf8(output.stderr)?,
    })
}

pub fn spawn_sync_with_timeout(cmd: &[&str], timeout_ms: u64) -> anyhow::Result<SpawnResult> {
    anyhow::ensure!(!cmd.is_empty(), "command is empty");
    if timeout_ms == 0 {
        return spawn_sync(cmd);
    }

    let mut child = Command::new(cmd[0])
        .args(&cmd[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        if child.try_wait()?.is_some() {
            let output = child.wait_with_output()?;
            return Ok(SpawnResult {
                exit_code: output.status.code(),
                signal: signal_code(&output.status),
                stdout: String::from_utf8(output.stdout)?,
                stderr: String::from_utf8(output.stderr)?,
            });
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let output = child.wait_with_output()?;
            let mut stderr = String::from_utf8(output.stderr)?;
            if !stderr.is_empty() && !stderr.ends_with('\n') {
                stderr.push('\n');
            }
            stderr.push_str(&format!("command timed out after {timeout_ms}ms"));
            return Ok(SpawnResult {
                exit_code: None,
                signal: signal_code(&output.status),
                stdout: String::from_utf8(output.stdout)?,
                stderr,
            });
        }
        sleep(Duration::from_millis(10));
    }
}

#[cfg(unix)]
fn signal_code(status: &std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[cfg(not(unix))]
fn signal_code(_status: &std::process::ExitStatus) -> Option<i32> {
    None
}

/// Outcome of a detached spawn attempt.
///
/// `Detached`           — child started in its own session/process group; parent
///                        SIGHUP propagation blocked (Unix `setsid()`) or new
///                        process group on Windows (`CREATE_NEW_PROCESS_GROUP`).
/// `NonDetachedFallback` — first-choice detach failed (e.g. `setsid()` EPERM in
///                        a Docker non-priv container). Child started without
///                        detach so it still runs, but the parent's SIGHUP can
///                        still kill it if the SessionStart hook returns first.
///                        Best-effort fallback per sh/ps1-absorption Issue 1.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetachOutcome {
    Detached,
    NonDetachedFallback,
}

/// Spawn a command in detached mode (no parent stdio, own session/group).
///
/// On Unix: closes stdin/stdout/stderr and calls `setsid()` via `pre_exec` so
/// the child becomes a new session leader. Parent SIGHUP (which fires when the
/// SessionStart hook exits) does not propagate to the child.
///
/// On Windows: uses `creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS)`
/// so the child runs without console attachment and ignores parent Ctrl+C
/// signals. The dropped `Child` handle releases the kernel object without
/// waiting.
///
/// Returns `Err` only when `Command::spawn` itself fails (e.g. binary missing,
/// permission denied). `setsid()` EPERM (Docker no-priv containers) surfaces
/// inside `pre_exec` and is intentionally NOT propagated — the child still
/// spawns; callers that need the EPERM signal should use
/// `spawn_detached_with_fallback` instead.
#[cfg(unix)]
pub fn spawn_detached(cmd: &[&str]) -> anyhow::Result<()> {
    use std::os::unix::process::CommandExt;
    anyhow::ensure!(!cmd.is_empty(), "command is empty");
    let mut command = Command::new(cmd[0]);
    command
        .args(&cmd[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    unsafe {
        command.pre_exec(|| {
            // Best-effort. setsid() can EPERM in Docker non-priv containers.
            // We swallow the error here — caller uses `spawn_detached_with_fallback`
            // when EPERM detection matters. Returning Ok keeps the child running
            // without setsid (parent SIGHUP risk accepted as fallback).
            let _ = libc::setsid();
            Ok(())
        });
    }
    let child = command.spawn()?;
    drop(child);
    Ok(())
}

#[cfg(windows)]
pub fn spawn_detached(cmd: &[&str]) -> anyhow::Result<()> {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, DETACHED_PROCESS};
    anyhow::ensure!(!cmd.is_empty(), "command is empty");
    let child = Command::new(cmd[0])
        .args(&cmd[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS)
        .spawn()?;
    drop(child);
    Ok(())
}

/// Spawn a command in detached mode, but fall back to a non-detached spawn
/// when the first-choice detach fails (Docker non-priv containers, etc.).
///
/// Always returns a `DetachOutcome` describing which path succeeded; only
/// returns `Err` when both the detached AND non-detached spawn fail (binary
/// missing, etc.).
///
/// sh/ps1-absorption Issue 1.1 (T1 codex tension): we deliberately do NOT
/// surface setsid() EPERM as an error and silently exit — that converts a
/// feature into a no-op for Docker users. Instead we retry without setsid so
/// the work still has a chance to complete inside the SessionStart hook
/// window. Callers that need observability into the fallback path can log
/// `DetachOutcome::NonDetachedFallback` to the hook event log.
#[cfg(unix)]
pub fn spawn_detached_with_fallback(cmd: &[&str]) -> anyhow::Result<DetachOutcome> {
    // Detect setsid EPERM proactively: if the process is already a session
    // leader (Docker no-priv often spawns processes that way), the kernel will
    // return EPERM. Probe with getsid(0) == getpid(); same-pid means we are
    // already a session leader and a subsequent setsid() would EPERM. In that
    // case skip the detach attempt and go straight to the fallback so the
    // outcome is observable to the caller.
    let already_session_leader = unsafe {
        let pid = libc::getpid();
        let sid = libc::getsid(0);
        sid == pid
    };
    if !already_session_leader {
        match spawn_detached(cmd) {
            Ok(()) => return Ok(DetachOutcome::Detached),
            Err(_) => { /* fall through to non-detached path */ }
        }
    }
    // Non-detached fallback. Child still runs without parent stdio, but no
    // setsid() — parent SIGHUP can propagate. Best-effort.
    use std::os::unix::process::CommandExt;
    anyhow::ensure!(!cmd.is_empty(), "command is empty");
    let mut command = Command::new(cmd[0]);
    command
        .args(&cmd[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // Detach from parent process group (best-effort) without becoming session
    // leader — equivalent to bash's `disown` semantics.
    unsafe {
        command.pre_exec(|| {
            let _ = libc::setpgid(0, 0);
            Ok(())
        });
    }
    let child = command.spawn()?;
    drop(child);
    Ok(DetachOutcome::NonDetachedFallback)
}

#[cfg(windows)]
pub fn spawn_detached_with_fallback(cmd: &[&str]) -> anyhow::Result<DetachOutcome> {
    // Match the Unix entry-point ensure! so callers don't depend on the inner
    // ensure inside the Err(_) fallback. Reviewer Issue 1 (PR #114): without
    // this, an empty cmd slice walks the Err path and only fails on the inner
    // ensure — the test contract ("both paths hit the same ensure") relies
    // on the front-loaded check matching Unix.
    anyhow::ensure!(!cmd.is_empty(), "command is empty");
    match spawn_detached(cmd) {
        Ok(()) => Ok(DetachOutcome::Detached),
        Err(_) => {
            let child = Command::new(cmd[0])
                .args(&cmd[1..])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            drop(child);
            Ok(DetachOutcome::NonDetachedFallback)
        }
    }
}

#[cfg(test)]
mod tests_detached {
    use super::*;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn lockfile_path(prefix: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("axhub-spawn-detached-{prefix}-{nonce}.flag"))
    }

    #[cfg(unix)]
    #[test]
    fn spawn_detached_runs_child_and_returns_immediately() {
        let lock = lockfile_path("unix-detach");
        let _ = std::fs::remove_file(&lock);
        let lock_str = lock.to_string_lossy().to_string();
        let start = SystemTime::now();
        spawn_detached(&["sh", "-c", &format!("sleep 1 && touch {lock_str}")])
            .expect("spawn_detached should not error for valid command");
        let parent_elapsed = SystemTime::now().duration_since(start).unwrap();
        assert!(
            parent_elapsed < Duration::from_millis(500),
            "parent should return immediately, elapsed: {parent_elapsed:?}"
        );
        // Poll up to 5s for the child lockfile.
        let deadline = SystemTime::now() + Duration::from_secs(5);
        while SystemTime::now() < deadline {
            if lock.exists() {
                let _ = std::fs::remove_file(&lock);
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = std::fs::remove_file(&lock);
        panic!("detached child did not create lockfile within 5s — detach failed");
    }

    #[cfg(unix)]
    #[test]
    fn spawn_detached_with_fallback_reports_outcome() {
        let lock = lockfile_path("unix-fallback");
        let _ = std::fs::remove_file(&lock);
        let lock_str = lock.to_string_lossy().to_string();
        let outcome =
            spawn_detached_with_fallback(&["sh", "-c", &format!("sleep 1 && touch {lock_str}")])
                .expect("fallback spawn should succeed");
        // Outcome MUST be one of the two enum variants; we don't assert which
        // because Docker / CI environments differ on whether setsid is allowed.
        assert!(matches!(
            outcome,
            DetachOutcome::Detached | DetachOutcome::NonDetachedFallback
        ));
        let deadline = SystemTime::now() + Duration::from_secs(5);
        while SystemTime::now() < deadline {
            if lock.exists() {
                let _ = std::fs::remove_file(&lock);
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = std::fs::remove_file(&lock);
        panic!("detached fallback child did not create lockfile within 5s");
    }

    #[test]
    fn spawn_detached_errors_on_empty_command() {
        let err = spawn_detached(&[]).unwrap_err();
        assert!(err.to_string().contains("command is empty"));
    }

    #[test]
    fn spawn_detached_with_fallback_errors_on_empty_command() {
        // empty input → both detached AND fallback path hit the same anyhow::ensure
        let err = spawn_detached_with_fallback(&[]).unwrap_err();
        assert!(err.to_string().contains("command is empty"));
    }
}
