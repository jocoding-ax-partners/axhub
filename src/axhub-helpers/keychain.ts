/**
 * keychain.ts — Phase 8 US-801: token discovery from OS keychain.
 *
 * Reads the axhub access_token that ax-hub-cli stores under
 * service="axhub" via zalando/keyring. Cross-platform shell-out:
 *   - macOS: `security find-generic-password -s axhub -w`
 *   - Linux: `secret-tool lookup service axhub`
 *   - Windows: deferred — AXHUB_TOKEN env var only
 *
 * Storage format (zalando/keyring): `go-keyring-base64:<base64 JSON>` where
 * the decoded JSON has `{schema_version, access_token, token_type,
 * expires_at, scopes}`. parseKeyringValue extracts access_token.
 *
 * Pure functions live here so tests can import without booting the helper
 * binary's main dispatch (importing index.ts triggers process.argv parsing).
 */

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
        return { error: "macOS keychain에 axhub token이 없어요. 'axhub auth login' 으로 한 번 로그인해주세요." };
      }
      const raw = (result.stdout?.toString() ?? "").trim();
      const token = parseKeyringValue(raw);
      if (token === null)
        return { error: "macOS keychain의 axhub token을 파싱할 수 없어요. axhub CLI 버전 확인 또는 'axhub auth login --force' 로 재발급 시도." };
      return { token, source: "macos-keychain" };
    } catch {
      return { error: "macOS 'security' 명령 실행 실패. /usr/bin/security 가 PATH에 있는지 확인해주세요." };
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
            "Linux secret-service에서 axhub token을 찾을 수 없어요. 'sudo apt-get install libsecret-tools' 로 secret-tool 설치 후 'axhub auth login' 또는 export AXHUB_TOKEN=... 로 우회해주세요.",
        };
      }
      const raw = (result.stdout?.toString() ?? "").trim();
      const token = parseKeyringValue(raw);
      if (token === null)
        return { error: "Linux secret-service의 axhub token을 파싱할 수 없어요. axhub CLI 재로그인 시도해주세요." };
      return { token, source: "linux-secret-service" };
    } catch {
      return {
        error:
          "secret-tool 명령이 PATH에 없어요. 'sudo apt-get install libsecret-tools' 후 다시 시도하시거나 export AXHUB_TOKEN=... 사용.",
      };
    }
  }
  if (platform === "win32") {
    return {
      error:
        "Windows에서는 현재 AXHUB_TOKEN 환경변수만 지원합니다. axhub auth login 후 토큰을 별도 안내받아 export AXHUB_TOKEN=... 으로 설정해주세요. PowerShell credential manager 통합은 다음 release에 추가될 예정입니다.",
    };
  }
  return { error: `지원하지 않는 플랫폼: ${platform}. AXHUB_TOKEN 환경변수로 우회해주세요.` };
};
