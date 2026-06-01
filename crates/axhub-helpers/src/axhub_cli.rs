use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Default timeout for short helper probes that shell out to the canonical
/// `axhub` CLI. Helpers should fail soft instead of hanging hook execution.
pub const DEFAULT_AXHUB_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
}

impl CliOutput {
    pub fn spawn_failed() -> Self {
        Self {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 127,
            timed_out: false,
        }
    }

    pub fn timed_out() -> Self {
        Self {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 124,
            timed_out: true,
        }
    }
}

pub fn axhub_bin_from_env() -> String {
    std::env::var("AXHUB_BIN").unwrap_or_else(|_| "axhub".to_string())
}

/// Kill the child and any descendants in its process group, then reap.
/// On Unix we send SIGKILL to the negative pgid (the whole group); on
/// other platforms we fall back to single-process kill on the child only.
fn kill_child_group(child: &mut std::process::Child) {
    #[cfg(unix)]
    {
        // SAFETY: the child was placed in its own process group via
        // `process_group(0)` immediately after spawn. Sending SIGKILL to
        // `-(pid)` reaches the entire group; on POSIX this is the canonical
        // way to clean up shell + sleep + any forked tools at once.
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

pub fn run_axhub(args: &[&str]) -> CliOutput {
    run_axhub_with_timeout(&axhub_bin_from_env(), args, DEFAULT_AXHUB_TIMEOUT)
}

fn axhub_command(axhub_bin: &str, args: &[&str], process_group: bool) -> Command {
    let mut cmd = Command::new(axhub_bin);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(unix)]
    if process_group {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    #[cfg(not(unix))]
    let _ = process_group;
    cmd
}

pub fn run_axhub_with_timeout(axhub_bin: &str, args: &[&str], timeout: Duration) -> CliOutput {
    // On Unix, place the child in its own process group so that any
    // grandchild it forks (sh -c "sleep 30" → sleep) can be killed
    // together via kill(-pgid). Without this, sh's child sleep inherits
    // the pipe write-end; when we kill sh on timeout, sleep keeps the
    // pipe open, the reader thread's read_to_end never sees EOF, and
    // the wall-clock blows past the timeout budget.
    let mut cmd = axhub_command(axhub_bin, args, true);
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(_) => {
            // Some sandboxed Linux runners reject `setpgid` during spawn and
            // report it as a spawn failure. Retry once without process-group
            // isolation so short, non-timeout probes still work; timeout paths
            // keep the process-group behavior whenever the platform permits it.
            let mut fallback = axhub_command(axhub_bin, args, false);
            match fallback.spawn() {
                Ok(child) => child,
                Err(_) => return CliOutput::spawn_failed(),
            }
        }
    };

    // Drain stdout/stderr in dedicated threads BEFORE polling try_wait.
    // Without this, output that exceeds the OS pipe buffer (~16 KB macOS,
    // ~64 KB Linux) blocks the child on write, the wait loop hits the
    // timeout, and the child gets killed with stdout silently truncated.
    // PR #149 / review #3: pipe-buffer deadlock corrupting trace evidence.
    let stdout_thread = child.stdout.take().map(|mut handle| {
        thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = handle.read_to_end(&mut buf);
            buf
        })
    });
    let stderr_thread = child.stderr.take().map(|mut handle| {
        thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = handle.read_to_end(&mut buf);
            buf
        })
    });

    let start = Instant::now();
    let (exit_code, timed_out) = loop {
        match child.try_wait() {
            Ok(Some(status)) => break (status.code().unwrap_or(127), false),
            Ok(None) if start.elapsed() >= timeout => {
                kill_child_group(&mut child);
                break (124, true);
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            // wait() machinery itself failed — treat as spawn failure.
            Err(_) => {
                kill_child_group(&mut child);
                break (127, false);
            }
        }
    };

    // Join reader threads. They drain via read_to_end and exit naturally
    // when the child closes its pipes (either on normal exit or on kill).
    let stdout = stdout_thread
        .and_then(|t| t.join().ok())
        .map(|v| String::from_utf8_lossy(&v).into_owned())
        .unwrap_or_default();
    let stderr = stderr_thread
        .and_then(|t| t.join().ok())
        .map(|v| String::from_utf8_lossy(&v).into_owned())
        .unwrap_or_default();

    CliOutput {
        stdout,
        stderr,
        exit_code,
        timed_out,
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    /// Write an executable shell script to `path` with `body` as the
    /// `#!/bin/sh` script body. Returns the path.
    fn write_script(path: &std::path::Path, body: &str) {
        let mut f = std::fs::File::create(path).expect("create script");
        writeln!(f, "#!/bin/sh\n{body}").expect("write script");
        let mut perms = std::fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms).unwrap();
    }

    #[test]
    fn run_axhub_drains_large_stdout_without_deadlock() {
        // 256 KB of stdout — comfortably above macOS (~16 KB) and Linux
        // (~64 KB) pipe-buffer thresholds. Without the reader-thread fix
        // this test hits the 5s timeout and returns truncated stdout.
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("big_stdout.sh");
        write_script(
            &script,
            "i=0\nwhile [ \"$i\" -lt 4096 ]; do\n  printf 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'\n  i=$((i + 1))\ndone",
        );

        let out = run_axhub_with_timeout(script.to_str().unwrap(), &[], Duration::from_secs(5));
        assert!(
            !out.timed_out,
            "large stdout must not deadlock the wait loop (exit_code={}, len={})",
            out.exit_code,
            out.stdout.len(),
        );
        assert_eq!(out.exit_code, 0);
        assert!(
            out.stdout.len() >= 262_144,
            "expected ≥256 KB stdout, got {} bytes",
            out.stdout.len()
        );
    }

    #[test]
    fn run_axhub_signals_timeout_on_slow_child() {
        // Pure timeout-classification check — partial-output recovery across
        // shells / pipe-orphans is intentionally not asserted because it's
        // flaky between sh/bash/dash + libc stdio buffering policies.
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("slow.sh");
        write_script(&script, "exec sleep 30");

        let out = run_axhub_with_timeout(script.to_str().unwrap(), &[], Duration::from_millis(300));
        assert!(out.timed_out);
        assert_eq!(out.exit_code, 124);
    }
}
