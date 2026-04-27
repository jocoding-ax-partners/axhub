/**
 * keychain-windows.ts — Windows Credential Manager bridge via PowerShell + PInvoke.
 *
 * Reads the axhub access_token that ax-hub-cli stores under TargetName="axhub"
 * via zalando/keyring's wincred provider. Strategy:
 *   1. spawn powershell.exe with inline Add-Type C# stub binding advapi32!CredReadW
 *   2. PS script writes ASCII sentinel to stdout: AXHUB_OK:<base64> | ERR:NOT_FOUND | ERR:LOAD_FAIL
 *   3. TS decodes base64 → UTF-8 → reuses parseKeyringValue from keychain.ts
 *
 * Sentinels are locale-independent (Korean Windows emits Korean .NET errors that
 * we never parse). EDR/AMSI detection: signalCode != null OR exit in {-1, 0xC0000409}.
 *
 * String.raw on PS_SCRIPT is mandatory — PowerShell uses $variables which would
 * otherwise interpolate as template literal placeholders.
 */

import type { KeychainResult } from "./keychain.ts";

export interface WindowsSpawnResult {
  exitCode: number;
  signalCode: string | undefined;
  stdout: string;
  stderr: string;
}

export type WindowsRunner = (cmd: string[], timeoutMs: number) => WindowsSpawnResult;

export const defaultWindowsRunner: WindowsRunner = (cmd, timeoutMs) => {
  const proc = Bun.spawnSync({
    cmd,
    stdout: "pipe",
    stderr: "pipe",
    timeout: timeoutMs,
  });
  return {
    exitCode: proc.exitCode ?? 1,
    signalCode: proc.signalCode,
    stdout: proc.stdout?.toString() ?? "",
    stderr: proc.stderr?.toString() ?? "",
  };
};

export const PS_SCRIPT = String.raw`$ErrorActionPreference = 'Stop'
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
}`;

const PS_TIMEOUT_MS = 8000;

const ERR_NOT_FOUND =
  "Windows Credential Manager에 axhub token이 없어요.\n" +
  "원인: 'axhub auth login' 미실행 또는 다른 사용자 계정으로 로그인됨.\n" +
  "해결: PowerShell에서 'axhub auth login' 실행 후 다시 시도해주세요.\n" +
  "다음: 그래도 안 되면 AXHUB_TOKEN 환경변수 우회 → $env:AXHUB_TOKEN='axhub_pat_...'";

const ERR_EXEC_POLICY =
  "잠깐만요.\n" +
  "Windows PowerShell 실행 정책 (ExecutionPolicy) 이 잠겨 있어요 (회사 GPO 가능성).\n" +
  "해결: AXHUB_TOKEN 환경변수로 우회 가능해요.\n" +
  "다음: PowerShell에서 $env:AXHUB_TOKEN='axhub_pat_...' 실행 후 token-init 재시도.";

const ERR_PINVOKE =
  "PowerShell 인라인 C# 컴파일 (Add-Type) 이 실패했어요.\n" +
  "원인: .NET Framework 누락 또는 PowerShell 5.1 미만 버전.\n" +
  "해결: AXHUB_TOKEN 환경변수가 정식 회피 경로예요.\n" +
  "다음: $env:AXHUB_TOKEN='axhub_pat_...' 후 token-init 재시도. PowerShell 5.1+ 권장.";

const ERR_EDR =
  "잠깐만요.\n" +
  "보안 솔루션 (V3, AhnLab, CrowdStrike 등) 이 PowerShell 호출을 차단했어요.\n" +
  "현재 v0.1.5 는 코드사이닝 전이라 EDR 가 PInvoke 패턴을 위협으로 분류해요 — 우리 책임이에요.\n" +
  "지금은 AXHUB_TOKEN 환경변수가 정식 회피 경로예요 ($env:AXHUB_TOKEN='axhub_pat_...').\n" +
  "v0.1.6 Authenticode 코드사이닝 후 EDR allowlist 가능해질 예정이에요.";

const ERR_SPAWN =
  "PowerShell 실행 자체가 실패했어요 (powershell.exe 가 PATH 에 없거나 권한 부족).\n" +
  "원인: Windows 최소 설치 환경 또는 Server Core.\n" +
  "해결: AXHUB_TOKEN 환경변수가 정식 회피 경로예요.\n" +
  "다음: $env:AXHUB_TOKEN='axhub_pat_...' 후 token-init 재시도.";

const isEdrSignal = (result: WindowsSpawnResult): boolean => {
  if (result.signalCode !== undefined) return true;
  // Windows STATUS_STACK_BUFFER_OVERRUN (0xC0000409) when AV terminates.
  // -1 used by some EDR vendors to signal forced termination.
  return result.exitCode === -1 || result.exitCode === 0xc0000409;
};

const decodeWindowsBlob = (b64: string): string | null => {
  try {
    return Buffer.from(b64, "base64").toString("utf8");
  } catch {
    return null;
  }
};

export const readWindowsKeychain = (
  runner: WindowsRunner = defaultWindowsRunner,
): KeychainResult => {
  let result: WindowsSpawnResult;
  try {
    result = runner(
      [
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
  } catch {
    return { error: ERR_SPAWN };
  }

  if (isEdrSignal(result)) return { error: ERR_EDR };

  const stdout = result.stdout.trim();
  const stderr = result.stderr;

  if (stdout === "ERR:NOT_FOUND") return { error: ERR_NOT_FOUND };
  if (stdout === "ERR:LOAD_FAIL") return { error: ERR_PINVOKE };

  // ExecutionPolicy block: PS exits non-zero, stderr typically mentions
  // "execution of scripts is disabled" (English) or AuthorizationManager
  // (any locale). We use the structural signal — exit != 0 AND no AXHUB_OK
  // sentinel — rather than parsing stderr text.
  if (!stdout.startsWith("AXHUB_OK:")) {
    if (
      stderr.includes("execution of scripts") ||
      stderr.includes("AuthorizationManager") ||
      stderr.includes("UnauthorizedAccess")
    ) {
      return { error: ERR_EXEC_POLICY };
    }
    return { error: ERR_SPAWN };
  }

  const b64 = stdout.slice("AXHUB_OK:".length).trim();
  const decoded = decodeWindowsBlob(b64);
  if (decoded === null) return { error: ERR_SPAWN };
  const token = defaultParse(decoded);
  if (token === null) return { error: ERR_NOT_FOUND };
  return { token, source: "windows-credential-manager" };
};

// Local copy of parseKeyringValue logic to avoid circular import.
// Kept in sync with keychain.ts:parseKeyringValue (single source of truth lives there;
// this is a defensive duplicate to break the import cycle keychain.ts ↔ keychain-windows.ts).
const defaultParse = (raw: string): string | null => {
  if (raw.length === 0) return null;
  const stripped = raw.startsWith("go-keyring-base64:")
    ? raw.slice("go-keyring-base64:".length)
    : raw;
  let decoded: string;
  try {
    decoded = Buffer.from(stripped.trim(), "base64").toString("utf8");
  } catch {
    return null;
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(decoded);
  } catch {
    return null;
  }
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) return null;
  const obj = parsed as Record<string, unknown>;
  const tok = obj["access_token"];
  if (typeof tok !== "string" || tok.length < 16) return null;
  return tok;
};
