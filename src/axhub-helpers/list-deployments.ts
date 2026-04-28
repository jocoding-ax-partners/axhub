/**
 * list-deployments.ts — REST API direct fallback for missing `axhub deploy list`.
 *
 * Phase 5 US-501: ax-hub-cli v0.1.x has no `deploy list` command. Plugin
 * cold-cache path (status/logs/recover skill first invocation, no
 * deployments.json yet) needs a way to discover deployment IDs without
 * forcing the user to hand-curl the API.
 *
 * Strategy: shell out to GET /api/v1/apps/{appID}/deployments?per_page=N with
 * Bearer token. Token discovery order:
 *   1. AXHUB_TOKEN env var
 *   2. ${XDG_CONFIG_HOME}/axhub-plugin/token file (mode 0600)
 *
 * Token source: `~/.config/axhub-plugin/token` (mode 0600), populated by
 * `axhub-helpers token-init` which reads ax-hub-cli's OS keychain entry
 * (macOS `security` / Linux `secret-tool` / Windows PowerShell + Advapi32
 * CredReadW). AXHUB_TOKEN env var overrides.
 *
 * On missing token: exit 65 (auth) with structured Korean recovery message.
 * On 404 app: exit 67 (not found). On other API error: exit 1.
 */

import { homedir } from "node:os";
import { join } from "node:path";
import { existsSync, readFileSync } from "node:fs";
import { createHash, X509Certificate } from "node:crypto";
import { connect } from "node:tls";

const DEFAULT_ENDPOINT = "https://hub-api.jocodingax.ai";
const HUB_API_HOST = "hub-api.jocodingax.ai";
const DEFAULT_LIMIT = 5;
const TLS_PIN_TIMEOUT_MS = 5_000;

export const HUB_API_SPKI_SHA256_PINS = [
  "sha256/vmsW4ExrgK3t3mFNtwk6KMsokm6PM+WNgC/KWhe7Z7g=",
] as const;

export const EXIT_LIST_OK = 0;
export const EXIT_LIST_AUTH = 65;
export const EXIT_LIST_NOT_FOUND = 67;
export const EXIT_LIST_TRANSPORT = 1;

export interface DeploymentSummary {
  id: number;
  app_id: number;
  status: string;          // pending|building|deploying|active|failed|stopped
  commit_sha: string;
  commit_message: string;
  branch: string;
  created_at: string;
}

export interface ListDeploymentsArgs {
  appId: string;          // numeric or slug — slug must already be resolved by caller
  limit?: number;
}

export interface ListDeploymentsResult {
  deployments: DeploymentSummary[];
  endpoint_used: string;
  exit_code: number;
  error_code?: string;
  error_message_kr?: string;
}

export class TlsPinError extends Error {
  constructor(message: string, readonly code = "security.tls_pin_failed") {
    super(message);
    this.name = "TlsPinError";
  }
}

export type TlsPinChecker = (endpoint: string) => Promise<void>;

const STATUS_MAP: Record<number, DeploymentSummary["status"]> = {
  0: "pending",
  1: "building",
  2: "deploying",
  3: "active",
  4: "failed",
  5: "stopped",
};

const tokenFromEnv = (): string | null => {
  const t = process.env["AXHUB_TOKEN"];
  return t && t.length > 0 ? t : null;
};

const tokenFromFile = (): string | null => {
  const xdg = process.env["XDG_CONFIG_HOME"];
  const dir = xdg && xdg.length > 0 ? join(xdg, "axhub-plugin") : join(homedir(), ".config", "axhub-plugin");
  const path = join(dir, "token");
  if (!existsSync(path)) return null;
  try {
    const t = readFileSync(path, "utf8").trim();
    return t.length > 0 ? t : null;
  } catch {
    return null;
  }
};

export const resolveToken = (): string | null => tokenFromEnv() ?? tokenFromFile();

const resolveEndpoint = (): string => {
  const e = process.env["AXHUB_ENDPOINT"];
  return e && e.length > 0 ? e : DEFAULT_ENDPOINT;
};

const proxyOverrideEnabled = (): boolean => process.env["AXHUB_ALLOW_PROXY"] === "1";

const pinnedHubApiUrl = (endpoint: string): URL | null => {
  let url: URL;
  try {
    url = new URL(endpoint);
  } catch {
    throw new TlsPinError(`잘못된 AXHUB_ENDPOINT 값이에요: ${endpoint}`, "security.endpoint_invalid");
  }

  if (url.hostname !== HUB_API_HOST) return null;
  if (url.protocol !== "https:") {
    throw new TlsPinError("hub-api.jocodingax.ai 는 HTTPS 로만 호출해야 해요.", "security.tls_required");
  }
  return url;
};

const spkiHashFromCert = (rawCert: Buffer): string => {
  const x509 = new X509Certificate(rawCert);
  const spkiDer = x509.publicKey.export({ type: "spki", format: "der" });
  return `sha256/${createHash("sha256").update(spkiDer).digest("base64")}`;
};

/**
 * PLAN row 60: pin the direct fallback's hub-api TLS identity.
 *
 * The primary plugin path delegates network calls to ax-hub-cli. This helper is
 * the one direct REST fallback, so it verifies the hub-api leaf public key
 * before sending the bearer token. Corporate TLS inspection can opt out with
 * AXHUB_ALLOW_PROXY=1, which is documented for org-admin rollout only.
 */
export async function verifyHubApiTlsPin(endpoint: string): Promise<void> {
  if (proxyOverrideEnabled()) return;

  const url = pinnedHubApiUrl(endpoint);
  if (url === null) return;

  const port = url.port.length > 0 ? Number(url.port) : 443;
  if (!Number.isInteger(port) || port <= 0) {
    throw new TlsPinError(`hub-api TLS 포트가 올바르지 않아요: ${url.port}`, "security.endpoint_invalid");
  }

  await new Promise<void>((resolve, reject) => {
    let settled = false;
    const socket = connect({
      host: url.hostname,
      port,
      servername: url.hostname,
      rejectUnauthorized: true,
      timeout: TLS_PIN_TIMEOUT_MS,
    });

    const finish = (err?: Error): void => {
      if (settled) return;
      settled = true;
      socket.destroy();
      if (err) reject(err);
      else resolve();
    };

    socket.once("secureConnect", () => {
      try {
        const cert = socket.getPeerCertificate(true);
        if (!cert.raw) {
          finish(new TlsPinError("hub-api 인증서 원본을 읽을 수 없어요."));
          return;
        }
        const actual = spkiHashFromCert(cert.raw);
        if (!HUB_API_SPKI_SHA256_PINS.includes(actual as (typeof HUB_API_SPKI_SHA256_PINS)[number])) {
          finish(new TlsPinError(`hub-api TLS pin mismatch: ${actual}`));
          return;
        }
        finish();
      } catch (e) {
        finish(e instanceof Error ? e : new TlsPinError("hub-api TLS pin 검증에 실패했어요."));
      }
    });

    socket.once("timeout", () => {
      finish(new TlsPinError("hub-api TLS pin 검증 시간이 초과됐어요.", "security.tls_pin_timeout"));
    });

    socket.once("error", (e) => {
      finish(e instanceof Error ? e : new TlsPinError("hub-api TLS 연결에 실패했어요."));
    });
  });
}

const parseAppId = (raw: string): number | null => {
  const n = parseInt(raw, 10);
  if (!Number.isFinite(n) || n <= 0) return null;
  return n;
};

interface BackendDeployment {
  id: number;
  app_id: number;
  status: number;
  commit_sha: string;
  commit_message?: string;
  branch?: string;
  created_at: string;
}

// Real API shape (verified live 2026-04-27 against /api/v1/apps/{id}/deployments):
//   { success: bool, data: { active_deployment: {...}, deployments: [...] }, meta: {...} }
// `deployments` array lives at data.deployments, NOT data itself.
interface BackendListEnvelope {
  success: boolean;
  data?: {
    active_deployment?: BackendDeployment;
    deployments?: BackendDeployment[];
  };
  error?: string;
  code?: string;
}

const buildAuthError = (): ListDeploymentsResult => ({
  deployments: [],
  endpoint_used: resolveEndpoint(),
  exit_code: EXIT_LIST_AUTH,
  error_code: "auth.token_missing",
  error_message_kr: "axhub 토큰을 찾을 수 없어요. 한 번만 로그인하시면 다음부터는 자동으로 작동해요:\n  axhub auth login\n또는 환경변수 우회: export AXHUB_TOKEN=axhub_pat_...",
});

export async function runListDeployments(
  args: ListDeploymentsArgs,
  fetchFn: typeof fetch = fetch,
  tlsPinChecker?: TlsPinChecker,
): Promise<ListDeploymentsResult> {
  const endpoint = resolveEndpoint();
  const token = resolveToken();
  if (token === null) return buildAuthError();

  const appId = parseAppId(args.appId);
  if (appId === null) {
    return {
      deployments: [],
      endpoint_used: endpoint,
      exit_code: EXIT_LIST_NOT_FOUND,
      error_code: "validation.app_id_invalid",
      error_message_kr: `앱 ID '${args.appId}' 형식이 잘못됐어요. 숫자만 입력해주세요. (slug 입력 시 axhub apps list 로 ID 확인)`,
    };
  }

  const limit = args.limit ?? DEFAULT_LIMIT;
  const url = `${endpoint}/api/v1/apps/${appId}/deployments?per_page=${limit}`;

  let resp: Response;
  try {
    const checker = tlsPinChecker ?? (fetchFn === fetch ? verifyHubApiTlsPin : undefined);
    await checker?.(endpoint);
    resp = await fetchFn(url, {
      method: "GET",
      headers: {
        Authorization: `Bearer ${token}`,
        Accept: "application/json",
      },
    });
  } catch (e) {
    return {
      deployments: [],
      endpoint_used: endpoint,
      exit_code: EXIT_LIST_TRANSPORT,
      error_code: e instanceof TlsPinError ? e.code : "transport.network_error",
      error_message_kr: e instanceof TlsPinError
        ? `axhub 서버 TLS 검증에 실패했어요. 신뢰 가능한 회사 proxy 환경이면 AXHUB_ALLOW_PROXY=1 을 설정하고, 그 외에는 네트워크를 확인해주세요. (${e.message})`
        : `axhub 서버까지 연결이 끊겼어요. 네트워크 확인 후 다시 시도해주세요. (${e instanceof Error ? e.message : "unknown"})`,
    };
  }

  if (resp.status === 401 || resp.status === 403) {
    return {
      deployments: [],
      endpoint_used: endpoint,
      exit_code: EXIT_LIST_AUTH,
      error_code: "auth.token_invalid",
      error_message_kr: "토큰이 만료되었거나 권한이 없어요. axhub auth login 으로 다시 인증해주세요.",
    };
  }

  if (resp.status === 404) {
    return {
      deployments: [],
      endpoint_used: endpoint,
      exit_code: EXIT_LIST_NOT_FOUND,
      error_code: "resource.app_not_found",
      error_message_kr: `app id ${appId} 를 찾을 수 없어요. axhub apps list 로 정확한 ID 확인해주세요.`,
    };
  }

  if (!resp.ok) {
    return {
      deployments: [],
      endpoint_used: endpoint,
      exit_code: EXIT_LIST_TRANSPORT,
      error_code: `http.${resp.status}`,
      error_message_kr: `서버 응답 에러 (HTTP ${resp.status}). 잠시 후 다시 시도해주세요.`,
    };
  }

  let env: BackendListEnvelope;
  try {
    env = (await resp.json()) as BackendListEnvelope;
  } catch (e) {
    return {
      deployments: [],
      endpoint_used: endpoint,
      exit_code: EXIT_LIST_TRANSPORT,
      error_code: "response.invalid_json",
      error_message_kr: `응답 파싱 실패. (${e instanceof Error ? e.message : "unknown"})`,
    };
  }

  const items = env.data?.deployments ?? [];
  const deployments: DeploymentSummary[] = items.map((d) => ({
    id: d.id,
    app_id: d.app_id,
    status: STATUS_MAP[d.status] ?? `unknown_${d.status}`,
    commit_sha: d.commit_sha,
    commit_message: d.commit_message ?? "",
    branch: d.branch ?? "",
    created_at: d.created_at,
  }));

  return {
    deployments,
    endpoint_used: endpoint,
    exit_code: EXIT_LIST_OK,
  };
}
