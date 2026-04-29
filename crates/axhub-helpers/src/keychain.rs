use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::keychain_windows::read_windows_keychain;

pub fn parse_keyring_value(raw: &str) -> Option<String> {
    if raw.is_empty() {
        return None;
    }
    let stripped = raw.strip_prefix("go-keyring-base64:").unwrap_or(raw).trim();
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(stripped)
        .ok()?;
    let parsed: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    let tok = parsed.get("access_token")?.as_str()?;
    (tok.len() >= 16).then(|| tok.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeychainResult {
    pub token: Option<String>,
    pub source: Option<String>,
    pub error: Option<String>,
}
impl KeychainResult {
    pub fn token(token: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            token: Some(token.into()),
            source: Some(source.into()),
            error: None,
        }
    }
    pub fn error(error: impl Into<String>) -> Self {
        Self {
            token: None,
            source: None,
            error: Some(error.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}
pub type CommandRunner = fn(&[&str], u64) -> CommandOutput;

pub fn default_runner(cmd: &[&str], _timeout_ms: u64) -> CommandOutput {
    match crate::spawn::spawn_sync(cmd) {
        Ok(r) => CommandOutput {
            exit_code: r.exit_code.unwrap_or(1),
            stdout: r.stdout,
            stderr: r.stderr,
        },
        Err(e) => CommandOutput {
            exit_code: 1,
            stdout: String::new(),
            stderr: e.to_string(),
        },
    }
}

pub fn read_keychain_token() -> KeychainResult {
    read_keychain_token_with_runner(std::env::consts::OS, default_runner)
}

pub fn read_keychain_token_with_runner(platform: &str, runner: CommandRunner) -> KeychainResult {
    match platform {
        "macos" => read_unix_keychain(&["security", "find-generic-password", "-s", "axhub", "-w"], 5000, "macos-keychain", runner,
            "잠깐만요.\n원인: macOS keychain 에 axhub token 이 저장돼 있지 않아요.\n해결: 'axhub auth login' 으로 한 번 로그인해주세요.\n다음: 로그인 후 token-init 자동 실행됩니다."),
        "linux" => read_unix_keychain(&["secret-tool", "lookup", "service", "axhub"], 5000, "linux-secret-service", runner,
            "잠깐만요.\n원인: Linux secret-service 에 axhub token 이 저장돼 있지 않거나 secret-tool 미설치.\n해결: 'sudo apt-get install libsecret-tools' 후 'axhub auth login' 실행.\n다음: 또는 AXHUB_TOKEN 환경변수로 우회 → export AXHUB_TOKEN=axhub_pat_..."),
        "windows" => read_windows_keychain(),
        other => KeychainResult::error(format!("잠깐만요.\n원인: 지원하지 않는 플랫폼이에요 (platform={other}).\n해결: AXHUB_TOKEN 환경변수로 우회 가능해요.\n다음: export AXHUB_TOKEN=axhub_pat_... 후 token-init 재시도.")),
    }
}

fn read_unix_keychain(
    cmd: &[&str],
    timeout_ms: u64,
    source: &str,
    runner: CommandRunner,
    not_found: &str,
) -> KeychainResult {
    let result = runner(cmd, timeout_ms);
    if result.exit_code != 0 {
        return KeychainResult::error(not_found);
    }
    match parse_keyring_value(result.stdout.trim()) {
        Some(token) => KeychainResult::token(token, source),
        None => KeychainResult::error("이상해요.\n원인: keychain 의 axhub token 형식을 파싱할 수 없어요 (axhub CLI 버전 mismatch 가능).\n해결: 'axhub auth login --force' 로 재발급 시도해주세요.\n다음: 그래도 안 되면 'axhub --version' 으로 CLI 버전 확인 후 업그레이드."),
    }
}
