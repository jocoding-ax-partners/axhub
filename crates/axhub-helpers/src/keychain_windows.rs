use base64::Engine;

use crate::keychain::{parse_keyring_value, KeychainResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsSpawnResult {
    pub exit_code: i32,
    pub signal_code: Option<String>,
    pub stdout: String,
    pub stderr: String,
}
pub type WindowsRunner = fn(&[&str], u64) -> WindowsSpawnResult;

pub const PS_TIMEOUT_MS: u64 = 8000;
pub const PS_SCRIPT: &str = r#"$ErrorActionPreference = 'Stop'
try {
  Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class Advapi32 {
  [DllImport("advapi32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
  public static extern bool CredReadW(string target, int type, int reservedFlag, out IntPtr CredentialPtr);
  [DllImport("advapi32.dll")]
  public static extern void CredFree(IntPtr cred);
  [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
  public struct CREDENTIAL {
    public int Flags;
    public int Type;
    public string TargetName;
    public string Comment;
    public long LastWritten;
    public int CredentialBlobSize;
    public IntPtr CredentialBlob;
    public int Persist;
    public int AttributeCount;
    public IntPtr Attributes;
    public string TargetAlias;
    public string UserName;
  }
}
"@ -PassThru | Out-Null
} catch {
  Write-Output 'ERR:LOAD_FAIL'
  exit 1
}

$ptr = [IntPtr]::Zero
$ok = [Advapi32]::CredReadW('axhub', 1, 0, [ref]$ptr)
if (-not $ok) {
  Write-Output 'ERR:NOT_FOUND'
  exit 0
}

try {
  $cred = [System.Runtime.InteropServices.Marshal]::PtrToStructure($ptr, [type][Advapi32+CREDENTIAL])
  $size = $cred.CredentialBlobSize
  if ($size -eq 0) {
    Write-Output 'ERR:NOT_FOUND'
    exit 0
  }
  $bytes = New-Object byte[] $size
  [System.Runtime.InteropServices.Marshal]::Copy($cred.CredentialBlob, $bytes, 0, $size)
  $b64 = [Convert]::ToBase64String($bytes)
  Write-Output ('AXHUB_OK:' + $b64)
} finally {
  [Advapi32]::CredFree($ptr)
}"#;

pub fn default_windows_runner(cmd: &[&str], _timeout_ms: u64) -> WindowsSpawnResult {
    match crate::spawn::spawn_sync(cmd) {
        Ok(r) => WindowsSpawnResult {
            exit_code: r.exit_code.unwrap_or(1),
            signal_code: r.signal.map(|s| s.to_string()),
            stdout: r.stdout,
            stderr: r.stderr,
        },
        Err(e) => WindowsSpawnResult {
            exit_code: 1,
            signal_code: None,
            stdout: String::new(),
            stderr: e.to_string(),
        },
    }
}

pub fn is_edr_signal(result: &WindowsSpawnResult) -> bool {
    result.signal_code.is_some()
        || result.exit_code == -1
        || result.exit_code == 0xC0000409u32 as i32
}
pub fn decode_windows_blob(b64: &str) -> Option<String> {
    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
}

pub fn read_windows_keychain() -> KeychainResult {
    read_windows_keychain_with_runner(default_windows_runner)
}

pub fn read_windows_keychain_with_runner(runner: WindowsRunner) -> KeychainResult {
    let result = runner(
        &[
            "powershell.exe",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            PS_SCRIPT,
        ],
        PS_TIMEOUT_MS,
    );
    if is_edr_signal(&result) {
        return KeychainResult::error("잠깐만요.\n보안 솔루션 (V3, AhnLab, CrowdStrike 등) 이 PowerShell 호출을 차단했어요.\n현재는 AXHUB_TOKEN 환경변수가 정식 회피 경로예요 ($env:AXHUB_TOKEN='axhub_pat_...').");
    }
    let stdout = result.stdout.trim();
    if stdout == "ERR:NOT_FOUND" {
        return KeychainResult::error("Windows Credential Manager에 axhub token이 없어요.\n원인: 'axhub auth login' 미실행 또는 다른 사용자 계정으로 로그인됨.\n해결: PowerShell에서 'axhub auth login' 실행 후 다시 시도해주세요.");
    }
    if stdout == "ERR:LOAD_FAIL" {
        return KeychainResult::error("PowerShell 인라인 C# 컴파일 (Add-Type) 이 실패했어요.\n원인: .NET Framework 누락 또는 PowerShell 5.1 미만 버전.\n해결: AXHUB_TOKEN 환경변수가 정식 회피 경로예요.");
    }
    if !stdout.starts_with("AXHUB_OK:") {
        if result.stderr.contains("execution of scripts")
            || result.stderr.contains("AuthorizationManager")
            || result.stderr.contains("UnauthorizedAccess")
        {
            return KeychainResult::error("잠깐만요.\nWindows PowerShell 실행 정책 (ExecutionPolicy) 이 잠겨 있어요 (회사 GPO 가능성).\n해결: AXHUB_TOKEN 환경변수로 우회 가능해요.");
        }
        return KeychainResult::error("PowerShell 실행 자체가 실패했어요 (powershell.exe 가 PATH 에 없거나 권한 부족).\n원인: Windows 최소 설치 환경 또는 Server Core.\n해결: AXHUB_TOKEN 환경변수가 정식 회피 경로예요.");
    }
    let Some(decoded) = decode_windows_blob(stdout.trim_start_matches("AXHUB_OK:").trim()) else {
        return KeychainResult::error("PowerShell 실행 자체가 실패했어요 (base64 decode).");
    };
    match parse_keyring_value(&decoded) {
        Some(token) => KeychainResult::token(token, "windows-credential-manager"),
        None => KeychainResult::error("Windows Credential Manager에 axhub token이 없어요."),
    }
}
