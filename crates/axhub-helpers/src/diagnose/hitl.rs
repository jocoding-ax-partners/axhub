//! Phase 1L HITL fallback — plan v6 §4.2.
//!
//! When the loop builder cannot synthesize a deterministic signal (no test
//! seam, no CLI replay, no trace), we fall back to asking the user. This
//! module replaces the previous bash + PowerShell template approach with a
//! single Rust subcommand so cross-platform parity is byte-identical (eng-review
//! architecture #4 BLOCKER).
//!
//! Wire:
//!   `axhub-helpers diagnose hitl --session <loop_id> --prompts <prompts.json>`
//! reads a list of [`PromptSpec`], drives the user through them, writes a
//! [`HitlResult`] JSON to `~/.axhub/loops/<loop_id>/captured.json`.
//!
//! Privacy: `run_from_files` enforces `crate::redact::redact_for_handoff` on
//! every capture value BEFORE writing to disk, and writes the output JSON
//! using `runtime_paths::write_private_file_no_follow` so the file lands at
//! mode 0o600 on Unix.

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::DiagnoseError;
use crate::runtime_paths::write_private_file_no_follow;
use crate::redact::redact_for_handoff;

/// Hard cap on the size of a `PromptSpec` file we will deserialise. Bounded
/// to prevent pathological JSON from DoS-ing the helper on a malicious
/// `prompts.json` write.
pub const PROMPTSPEC_BYTE_CAP: usize = 1024 * 1024;

/// One prompt step. `Step` shows a message and waits for Enter. `Capture`
/// collects a free-text response into `key`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PromptKind {
    Step,
    Capture { key: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptSpec {
    #[serde(flatten)]
    pub kind: PromptKind,
    pub message: String,
    /// Per-prompt timeout in seconds (plan v6 §4.2 default 60).
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    /// Capture byte cap (plan v6 default 102_400). Ignored for `Step`.
    #[serde(default = "default_max_bytes")]
    pub max_bytes: usize,
}

fn default_timeout_secs() -> u64 {
    60
}

fn default_max_bytes() -> usize {
    100 * 1024
}

/// Aggregated outcome of a HITL run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HitlResult {
    /// Capture key → UTF-8 value (post-truncation, pre-redaction).
    pub captures: std::collections::BTreeMap<String, String>,
    /// Keys whose prompt timed out.
    pub timed_out: Vec<String>,
    /// Keys whose response was truncated at `max_bytes`.
    pub truncated: Vec<String>,
}

/// Session-wide timeout (plan v6 §4.2 default 300s).
pub const HITL_SESSION_TIMEOUT_SECS: u64 = 300;

/// Run a HITL session over `prompts`, calling `runner` for each prompt. The
/// runner abstraction lets tests inject a deterministic prompt provider while
/// production code wires it to crossterm + tokio stdio.
pub fn run_with_runner(
    prompts: &[PromptSpec],
    runner: &mut dyn PromptRunner,
) -> Result<HitlResult, DiagnoseError> {
    let mut result = HitlResult::default();
    let session_deadline =
        std::time::Instant::now() + Duration::from_secs(HITL_SESSION_TIMEOUT_SECS);
    for prompt in prompts {
        if std::time::Instant::now() >= session_deadline {
            // Session timeout — abort remaining prompts but preserve partial result.
            return Err(DiagnoseError::HitlAborted(format!(
                "session timeout after {HITL_SESSION_TIMEOUT_SECS}s; {} prompts unfilled",
                prompts.len() - result.captures.len()
            )));
        }
        match &prompt.kind {
            PromptKind::Step => match runner.step(&prompt.message, prompt.timeout_secs) {
                StepOutcome::Ok => {}
                StepOutcome::TimedOut => result.timed_out.push("__step__".into()),
                StepOutcome::Aborted => {
                    return Err(DiagnoseError::HitlAborted("step aborted by runner".into()));
                }
            },
            PromptKind::Capture { key } => {
                match runner.capture(&prompt.message, prompt.timeout_secs, prompt.max_bytes) {
                    CaptureOutcome::Ok { value, truncated } => {
                        if truncated {
                            result.truncated.push(key.clone());
                        }
                        result.captures.insert(key.clone(), value);
                    }
                    CaptureOutcome::TimedOut => {
                        result.timed_out.push(key.clone());
                        result.captures.insert(key.clone(), String::new());
                    }
                    CaptureOutcome::Aborted => {
                        return Err(DiagnoseError::HitlAborted(format!(
                            "capture for {key} aborted"
                        )));
                    }
                }
            }
        }
    }
    Ok(result)
}

/// Parse spec from disk, run it through the given runner, write result JSON.
/// Used by the `diagnose hitl` subcommand entry point.
///
/// Two privacy invariants are enforced here, NOT delegated to the caller:
///   1. Every capture value passes through `redact_for_handoff` before it is
///      persisted (sk-/ghp_/Bearer/AWS/PEM/url-creds patterns are masked).
///   2. The output JSON is written via `write_private_file_no_follow`, so
///      the file is mode 0o600 on Unix and refuses to follow a pre-existing
///      symlink at the output path.
pub fn run_from_files(
    spec_path: &Path,
    output_path: &Path,
    runner: &mut dyn PromptRunner,
) -> Result<HitlResult, DiagnoseError> {
    let raw = std::fs::read(spec_path)?;
    if raw.len() > PROMPTSPEC_BYTE_CAP {
        return Err(DiagnoseError::HitlAborted(format!(
            "PromptSpec file exceeds {PROMPTSPEC_BYTE_CAP} byte cap ({} bytes)",
            raw.len()
        )));
    }
    let prompts: Vec<PromptSpec> = serde_json::from_slice(&raw)?;
    let mut result = run_with_runner(&prompts, runner)?;
    let mut new_truncations: Vec<String> = Vec::new();
    for (key, value) in result.captures.iter_mut() {
        let (red, truncated) = redact_for_handoff(value);
        *value = red;
        if truncated && !result.truncated.iter().any(|k| k == key) {
            new_truncations.push(key.clone());
        }
    }
    result.truncated.extend(new_truncations);
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_vec_pretty(&result)?;
    write_private_file_no_follow(output_path, &json).map_err(|e| {
        DiagnoseError::HitlAborted(format!("private write of captured.json failed: {e}"))
    })?;
    Ok(result)
}

/// Outcome of one `step` prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepOutcome {
    Ok,
    TimedOut,
    Aborted,
}

/// Outcome of one `capture` prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureOutcome {
    Ok { value: String, truncated: bool },
    TimedOut,
    Aborted,
}

/// Trait for I/O backends. Production uses `StdioRunner` (crossterm + stdin);
/// tests use `FakeRunner`.
pub trait PromptRunner {
    fn step(&mut self, message: &str, timeout_secs: u64) -> StepOutcome;
    fn capture(&mut self, message: &str, timeout_secs: u64, max_bytes: usize) -> CaptureOutcome;
}

/// Production runner — stdin/stdout backed by `crossterm` for the
/// `diagnose hitl` subcommand. Reads a single line per capture (Enter to
/// submit, byte-cap enforced after the line is read).
///
/// We deliberately keep this small: the orchestrator already validated the
/// prompt list and the redaction + private-write happen one layer up in
/// `run_from_files`.
pub struct StdioRunner;

impl StdioRunner {
    pub fn new() -> Self {
        Self
    }

    fn read_line_with_timeout(timeout: Duration) -> Option<String> {
        // Non-blocking poll on stdin via crossterm event loop. We use blocking
        // stdin read inside a thread, with a deadline check so a stuck reader
        // returns None rather than hanging forever.
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        std::thread::spawn(move || {
            let mut buf = String::new();
            if std::io::BufRead::read_line(&mut std::io::stdin().lock(), &mut buf).is_ok() {
                // Trim the trailing newline that `read_line` keeps.
                if buf.ends_with('\n') {
                    buf.pop();
                    if buf.ends_with('\r') {
                        buf.pop();
                    }
                }
                let _ = tx.send(buf);
            }
        });
        rx.recv_timeout(timeout).ok()
    }
}

impl Default for StdioRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptRunner for StdioRunner {
    fn step(&mut self, message: &str, timeout_secs: u64) -> StepOutcome {
        println!("{message}");
        println!("(Enter 누르면 다음으로 진행해요)");
        match Self::read_line_with_timeout(Duration::from_secs(timeout_secs)) {
            Some(_) => StepOutcome::Ok,
            None => StepOutcome::TimedOut,
        }
    }
    fn capture(&mut self, message: &str, timeout_secs: u64, max_bytes: usize) -> CaptureOutcome {
        println!("{message}");
        let Some(line) = Self::read_line_with_timeout(Duration::from_secs(timeout_secs)) else {
            return CaptureOutcome::TimedOut;
        };
        let (value, truncated) = if line.len() > max_bytes {
            let mut cut = max_bytes;
            while cut > 0 && !line.is_char_boundary(cut) {
                cut -= 1;
            }
            (line[..cut].to_string(), true)
        } else {
            (line, false)
        };
        CaptureOutcome::Ok { value, truncated }
    }
}

/// Test fixture: returns canned answers in order. `step` always Ok; capture
/// pulls from the queue.
pub struct FakeRunner {
    pub answers: std::collections::VecDeque<String>,
}

impl FakeRunner {
    pub fn new<I, S>(answers: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            answers: answers.into_iter().map(Into::into).collect(),
        }
    }
}

impl PromptRunner for FakeRunner {
    fn step(&mut self, _message: &str, _timeout_secs: u64) -> StepOutcome {
        StepOutcome::Ok
    }
    fn capture(&mut self, _message: &str, _timeout_secs: u64, max_bytes: usize) -> CaptureOutcome {
        let Some(mut value) = self.answers.pop_front() else {
            return CaptureOutcome::TimedOut;
        };
        let truncated = if value.len() > max_bytes {
            // Truncate at UTF-8 char boundary.
            let mut cut = max_bytes;
            while cut > 0 && !value.is_char_boundary(cut) {
                cut -= 1;
            }
            value.truncate(cut);
            true
        } else {
            false
        };
        CaptureOutcome::Ok { value, truncated }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn cap(key: &str, message: &str) -> PromptSpec {
        PromptSpec {
            kind: PromptKind::Capture {
                key: key.to_string(),
            },
            message: message.to_string(),
            timeout_secs: 60,
            max_bytes: 1024,
        }
    }

    #[test]
    fn captures_round_trip_via_fake() {
        let prompts = vec![
            cap("err_msg", "Paste the error"),
            cap("user_choice", "Retry?"),
        ];
        let mut runner = FakeRunner::new(["npm ERR! EACCES", "yes"]);
        let result = run_with_runner(&prompts, &mut runner).unwrap();
        assert_eq!(result.captures.len(), 2);
        assert_eq!(result.captures["err_msg"], "npm ERR! EACCES");
        assert_eq!(result.captures["user_choice"], "yes");
        assert!(result.timed_out.is_empty());
        assert!(result.truncated.is_empty());
    }

    #[test]
    fn step_kind_passes_through() {
        let prompts = vec![PromptSpec {
            kind: PromptKind::Step,
            message: "Run `axhub deploy`".into(),
            timeout_secs: 60,
            max_bytes: 0,
        }];
        let mut runner = FakeRunner::new(Vec::<String>::new());
        let result = run_with_runner(&prompts, &mut runner).unwrap();
        assert!(result.captures.is_empty());
        assert!(result.timed_out.is_empty());
    }

    #[test]
    fn capture_byte_cap_truncates() {
        let prompts = vec![PromptSpec {
            kind: PromptKind::Capture { key: "big".into() },
            message: "paste".into(),
            timeout_secs: 60,
            max_bytes: 32,
        }];
        let huge = "x".repeat(100);
        let mut runner = FakeRunner::new([huge]);
        let result = run_with_runner(&prompts, &mut runner).unwrap();
        assert_eq!(result.captures["big"].len(), 32);
        assert_eq!(result.truncated, vec!["big".to_string()]);
    }

    #[test]
    fn missing_answer_logs_timeout() {
        let prompts = vec![cap("ghost", "this one")];
        let mut runner = FakeRunner::new(Vec::<String>::new());
        let result = run_with_runner(&prompts, &mut runner).unwrap();
        assert_eq!(result.timed_out, vec!["ghost".to_string()]);
        assert_eq!(result.captures["ghost"], "");
    }

    #[test]
    fn korean_unicode_preserved() {
        let prompts = vec![cap("msg", "안녕")];
        let mut runner = FakeRunner::new(["에러: 권한 없음"]);
        let result = run_with_runner(&prompts, &mut runner).unwrap();
        assert_eq!(result.captures["msg"], "에러: 권한 없음");
    }

    #[test]
    fn run_from_files_round_trips_to_disk() {
        let dir = TempDir::new().unwrap();
        let spec = dir.path().join("prompts.json");
        let out = dir.path().join("captured.json");
        let prompts = vec![cap("err", "paste")];
        std::fs::write(&spec, serde_json::to_vec(&prompts).unwrap()).unwrap();
        let mut runner = FakeRunner::new(["disk-test"]);
        let result = run_from_files(&spec, &out, &mut runner).unwrap();
        assert_eq!(result.captures["err"], "disk-test");
        // Persisted JSON readable + matches.
        let raw = std::fs::read(&out).unwrap();
        let parsed: HitlResult = serde_json::from_slice(&raw).unwrap();
        assert_eq!(parsed.captures["err"], "disk-test");
    }

    #[test]
    fn run_from_files_redacts_secrets_before_disk_write() {
        // A vibe coder pasting an OpenAI key or a Bearer token into the
        // HITL prompt MUST NOT see that secret persisted in plaintext on
        // disk. This pins the redact_for_handoff invariant at the boundary.
        let dir = TempDir::new().unwrap();
        let spec = dir.path().join("prompts.json");
        let out = dir.path().join("captured.json");
        let prompts = vec![cap("err", "paste the error")];
        std::fs::write(&spec, serde_json::to_vec(&prompts).unwrap()).unwrap();
        let leaky = "trace: API failed using sk-abcdefghij0123456789ZZZZ here";
        let mut runner = FakeRunner::new([leaky]);
        let result = run_from_files(&spec, &out, &mut runner).unwrap();
        assert!(
            result.captures["err"].contains("<REDACTED_OPENAI_KEY>"),
            "in-memory capture must be redacted: {}",
            result.captures["err"]
        );
        let raw = std::fs::read_to_string(&out).unwrap();
        assert!(
            !raw.contains("sk-abcdefghij0123456789ZZZZ"),
            "raw OpenAI key leaked to disk: {raw}"
        );
        assert!(
            raw.contains("<REDACTED_OPENAI_KEY>"),
            "redaction marker missing from on-disk JSON: {raw}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn run_from_files_writes_with_private_mode() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let spec = dir.path().join("prompts.json");
        let out = dir.path().join("captured.json");
        let prompts = vec![cap("err", "paste")];
        std::fs::write(&spec, serde_json::to_vec(&prompts).unwrap()).unwrap();
        let mut runner = FakeRunner::new(["disk-test"]);
        run_from_files(&spec, &out, &mut runner).unwrap();
        let meta = std::fs::metadata(&out).unwrap();
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "captured.json must be mode 0o600, got {mode:o}"
        );
    }

    #[test]
    fn run_from_files_rejects_oversize_spec() {
        let dir = TempDir::new().unwrap();
        let spec = dir.path().join("prompts.json");
        let out = dir.path().join("captured.json");
        let huge: Vec<u8> = vec![b'x'; PROMPTSPEC_BYTE_CAP + 1];
        std::fs::write(&spec, &huge).unwrap();
        let mut runner = FakeRunner::new(Vec::<String>::new());
        let err = run_from_files(&spec, &out, &mut runner).unwrap_err();
        assert!(
            matches!(err, DiagnoseError::HitlAborted(_)),
            "expected HitlAborted on oversize spec: {err:?}"
        );
    }

    #[test]
    fn promptspec_json_deserializes_step_and_capture() {
        let json = r#"[
            {"kind":"step","message":"do X","timeout_secs":30,"max_bytes":0},
            {"kind":"capture","key":"v","message":"paste","timeout_secs":15,"max_bytes":1024}
        ]"#;
        let parsed: Vec<PromptSpec> = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.len(), 2);
        assert!(matches!(parsed[0].kind, PromptKind::Step));
        match &parsed[1].kind {
            PromptKind::Capture { key } => assert_eq!(key, "v"),
            _ => panic!("expected Capture"),
        }
    }
}
