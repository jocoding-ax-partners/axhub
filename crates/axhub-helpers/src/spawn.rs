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
