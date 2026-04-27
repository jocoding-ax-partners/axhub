/**
 * keychain.ts — token discovery from OS keychain.
 *
 * Reads the axhub access_token that ax-hub-cli stores under service="axhub"
 * via zalando/keyring. macOS: `security find-generic-password -s axhub -w`;
 * Linux: `secret-tool lookup service axhub`; Windows: PowerShell + Add-Type
 * PInvoke against advapi32!CredReadW (see ./keychain-windows.ts).
 *
 * Storage format: `go-keyring-base64:<base64 JSON>` where decoded JSON has
 * `{schema_version, access_token, token_type, expires_at, scopes}`.
 *
 * Pure functions live here so tests can import without booting the helper
 * binary's main dispatch (importing index.ts triggers process.argv parsing).
 */

import { readWindowsKeychain } from "./keychain-windows.ts";

export const parseKeyringValue = (raw: string): string | null => {
  if (raw.length === 0) return null;
  const stripped = raw.startsWith("go-keyring-base64:") ? raw.slice("go-keyring-base64:".length) : raw;
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

export interface KeychainResult {
  token?: string;
  source?: string;
  error?: string;
}

export const readKeychainToken = (): KeychainResult => {
  const platform = process.platform;
  if (platform === "darwin") {
    try {
      const result = Bun.spawnSync({
        cmd: ["security", "find-generic-password", "-s", "axhub", "-w"],
        stdout: "pipe",
        stderr: "pipe",
        timeout: 5000,
      });
      if (result.exitCode !== 0) {
        return {
          error:
            "잠깐만요.\n" +
            "원인: macOS keychain 에 axhub token 이 저장돼 있지 않아요.\n" +
            "해결: 'axhub auth login' 으로 한 번 로그인해주세요.\n" +
            "다음: 로그인 후 token-init 자동 실행됩니다.",
        };
      }
      const raw = (result.stdout?.toString() ?? "").trim();
      const token = parseKeyringValue(raw);
      if (token === null)
        return {
          error:
            "이상해요.\n" +
            "원인: macOS keychain 의 axhub token 형식을 파싱할 수 없어요 (axhub CLI 버전 mismatch 가능).\n" +
            "해결: 'axhub auth login --force' 로 재발급 시도해주세요.\n" +
            "다음: 그래도 안 되면 'axhub --version' 으로 CLI 버전 확인 후 업그레이드.",
        };
      return { token, source: "macos-keychain" };
    } catch {
      return {
        error:
          "잠깐만요.\n" +
          "원인: macOS 'security' 명령 실행 자체가 실패했어요.\n" +
          "해결: /usr/bin/security 가 PATH 에 있는지 확인해주세요.\n" +
          "다음: 또는 AXHUB_TOKEN 환경변수로 우회 → export AXHUB_TOKEN=axhub_pat_...",
      };
    }
  }
  if (platform === "linux") {
    try {
      const result = Bun.spawnSync({
        cmd: ["secret-tool", "lookup", "service", "axhub"],
        stdout: "pipe",
        stderr: "pipe",
        timeout: 5000,
      });
      if (result.exitCode !== 0) {
        return {
          error:
            "잠깐만요.\n" +
            "원인: Linux secret-service 에 axhub token 이 저장돼 있지 않거나 secret-tool 미설치.\n" +
            "해결: 'sudo apt-get install libsecret-tools' 후 'axhub auth login' 실행.\n" +
            "다음: 또는 AXHUB_TOKEN 환경변수로 우회 → export AXHUB_TOKEN=axhub_pat_...",
        };
      }
      const raw = (result.stdout?.toString() ?? "").trim();
      const token = parseKeyringValue(raw);
      if (token === null)
        return {
          error:
            "이상해요.\n" +
            "원인: Linux secret-service 의 axhub token 형식을 파싱할 수 없어요 (axhub CLI 버전 mismatch 가능).\n" +
            "해결: 'axhub auth login --force' 로 재발급 시도해주세요.\n" +
            "다음: 그래도 안 되면 'axhub --version' 으로 CLI 버전 확인 후 업그레이드.",
        };
      return { token, source: "linux-secret-service" };
    } catch {
      return {
        error:
          "잠깐만요.\n" +
          "원인: secret-tool 명령이 PATH 에 없거나 D-Bus session bus 미실행.\n" +
          "해결: 'sudo apt-get install libsecret-tools' + 'eval $(dbus-launch --sh-syntax)' 시도.\n" +
          "다음: 또는 AXHUB_TOKEN 환경변수로 우회 → export AXHUB_TOKEN=axhub_pat_...",
      };
    }
  }
  if (platform === "win32") return readWindowsKeychain();
  return {
    error:
      "잠깐만요.\n" +
      `원인: 지원하지 않는 플랫폼이에요 (platform=${platform}).\n` +
      "해결: AXHUB_TOKEN 환경변수로 우회 가능해요.\n" +
      "다음: export AXHUB_TOKEN=axhub_pat_... 후 token-init 재시도.",
  };
};
