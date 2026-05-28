use std::process::{Command, Stdio};
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

pub fn run_axhub(args: &[&str]) -> CliOutput {
    run_axhub_with_timeout(&axhub_bin_from_env(), args, DEFAULT_AXHUB_TIMEOUT)
}

pub fn run_axhub_with_timeout(axhub_bin: &str, args: &[&str], timeout: Duration) -> CliOutput {
    let mut child = match Command::new(axhub_bin)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return CliOutput::spawn_failed(),
    };

    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return match child.wait_with_output() {
                    Ok(output) => CliOutput {
                        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                        exit_code: output.status.code().unwrap_or(127),
                        timed_out: false,
                    },
                    Err(_) => CliOutput::spawn_failed(),
                };
            }
            Ok(None) if start.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return CliOutput::timed_out();
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(_) => return CliOutput::spawn_failed(),
        }
    }
}
