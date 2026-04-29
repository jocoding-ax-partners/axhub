use std::process::Command;

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

#[cfg(unix)]
fn signal_code(status: &std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[cfg(not(unix))]
fn signal_code(_status: &std::process::ExitStatus) -> Option<i32> {
    None
}
