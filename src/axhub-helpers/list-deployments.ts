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

const DEFAULT_ENDPOINT = "https://hub-api.jocodingax.ai";
const DEFAULT_LIMIT = 5;

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

interface BackendListEnvelope {
  success: boolean;
  data?: BackendDeployment[];
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
      error_code: "transport.network_error",
      error_message_kr: `axhub 서버까지 연결이 끊겼어요. 네트워크 확인 후 다시 시도해주세요. (${e instanceof Error ? e.message : "unknown"})`,
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

  const items = env.data ?? [];
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
