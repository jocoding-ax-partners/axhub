/**
 * preflight.ts — CLI version range gate + auth status pre-flight (US-001).
 *
 * Implements PLAN audit row 14 (MIN_CLI_VERSION semver gate) and the
 * `preflight` subcommand consumed by skills/deploy/SKILL.md step 2.
 *
 * Design notes
 * ------------
 * - axhub v0.1.0 prints `--version` as plain text only ("axhub 0.1.0 (commit
 *   ...)"); no `--json` form exists. We extract semver via regex.
 * - `axhub auth status --json` returns {user_email, user_id, expires_at,
 *   scopes} on success, or {code, detail, ...} on misconfiguration — both with
 *   exit 0. We discriminate via the `code` field, not exit code.
 * - Profile + endpoint live in env vars or ~/.config/axhub/config.yaml; the
 *   auth status JSON does NOT echo them. We surface env values when present
 *   (sufficient for the deploy preview card; CLI fills the rest itself).
 * - Tests inject a fake runner so we never depend on a real `axhub` binary
 *   or PATH state during CI.
 */

import { existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

import semver from "semver";

export const MIN_AXHUB_CLI_VERSION = "0.1.0";
export const MAX_AXHUB_CLI_VERSION = "0.2.0"; // exclusive (audit row 49)

const LAST_DEPLOY_CACHE = join(homedir(), ".cache", "axhub-plugin", "last-deploy.json");

// Exit code constants (PLAN §3.2 contract).
export const EXIT_OK = 0;
export const EXIT_USAGE = 64;
export const EXIT_AUTH = 65;

export interface SpawnResult {
  exitCode: number;
  stdout: string;
  stderr: string;
}

export type CommandRunner = (cmd: string[]) => SpawnResult;

/**
 * Default runner — uses Bun.spawnSync. Tests pass a stub instead so they
 * never touch the real binary.
 */
export const defaultRunner: CommandRunner = (cmd) => {
  const proc = Bun.spawnSync({ cmd, stdout: "pipe", stderr: "pipe" });
  return {
    // exitCode is `number | null` while the process is in flight; sync return
    // means it's settled, but coerce defensively so downstream is always int.
    exitCode: proc.exitCode ?? 1,
    stdout: proc.stdout.toString(),
    stderr: proc.stderr.toString(),
  };
};

/**
 * Resolve the axhub binary path. Tests override via AXHUB_BIN for
 * integration smoke tests; preflight unit tests use the runner injection
 * instead.
 */
export const axhubBin = (): string => process.env["AXHUB_BIN"] || "axhub";

/**
 * Extract the first semver triple from arbitrary text. axhub v0.1.0 prints
 * `axhub 0.1.0 (commit ..., built ..., darwin/arm64)`; we take the first
 * `\d+\.\d+\.\d+` match. Returns null when no semver-shaped substring is
 * present (e.g. spawn failure produced empty stdout).
 */
export function extractSemver(text: string): string | null {
  const match = text.match(/(\d+)\.(\d+)\.(\d+)/);
  return match ? `${match[1]}.${match[2]}.${match[3]}` : null;
}

interface AuthStatusOk {
  ok: true;
  user_email: string;
  user_id: number;
  expires_at: string;
  scopes: string[];
}

interface AuthStatusError {
  ok: false;
  code: string;
  detail: string;
}

export type AuthStatus = AuthStatusOk | AuthStatusError;

/**
 * Parse `axhub auth status --json` output. The CLI returns either a success
 * object or an error object (with exit 0 in both cases) — we discriminate
 * via the `code` field.
 */
export function parseAuthStatus(stdout: string): AuthStatus {
  let parsed: unknown;
  try {
    parsed = JSON.parse(stdout);
  } catch {
    return { ok: false, code: "parse_error", detail: "auth status returned non-JSON" };
  }
  if (parsed && typeof parsed === "object") {
    const obj = parsed as Record<string, unknown>;
    if (typeof obj["code"] === "string") {
      return {
        ok: false,
        code: obj["code"],
        detail: typeof obj["detail"] === "string" ? obj["detail"] : "",
      };
    }
    if (typeof obj["user_email"] === "string" && Array.isArray(obj["scopes"])) {
      return {
        ok: true,
        user_email: obj["user_email"],
        user_id: typeof obj["user_id"] === "number" ? obj["user_id"] : 0,
        expires_at: typeof obj["expires_at"] === "string" ? obj["expires_at"] : "",
        scopes: obj["scopes"].filter((s): s is string => typeof s === "string"),
      };
    }
  }
  return {
    ok: false,
    code: "unknown_shape",
    detail: "auth status JSON missing expected fields",
  };
}

export interface PreflightOutput {
  cli_version: string | null;
  in_range: boolean;
  cli_too_old: boolean;
  cli_too_new: boolean;
  cli_present: boolean;
  auth_ok: boolean;
  auth_error_code: string | null;
  scopes: string[];
  profile: string | null;
  endpoint: string | null;
  user_email: string | null;
  expires_at: string | null;
  // Phase 17 US-1706 — !command injection context for vibe coder UX.
  // current_app reads $AXHUB_APP_SLUG (set by deploy/recover write-back);
  // last_deploy_* read from ~/.cache/axhub-plugin/last-deploy.json (written by C7).
  current_app: string | null;
  current_env: string | null;
  last_deploy_id: string | null;
  last_deploy_status: string | null;
  plugin_version: string;
}

interface LastDeployCache {
  deployment_id: string;
  status: string;
  app_slug?: string;
}

function readLastDeployCache(): LastDeployCache | null {
  if (!existsSync(LAST_DEPLOY_CACHE)) return null;
  try {
    const parsed: unknown = JSON.parse(readFileSync(LAST_DEPLOY_CACHE, "utf8"));
    if (parsed && typeof parsed === "object") {
      const obj = parsed as Record<string, unknown>;
      const id = obj["deployment_id"];
      const status = obj["status"];
      if (typeof id === "string" && typeof status === "string") {
        return {
          deployment_id: id,
          status,
          app_slug: typeof obj["app_slug"] === "string" ? obj["app_slug"] : undefined,
        };
      }
    }
  } catch {
    // Corrupt cache — treat as absent. Cache writer (C7) writes atomically.
  }
  return null;
}

/**
 * Run preflight against a (possibly fake) runner. Pure-ish: no console output,
 * no process.exit. Returns the structured output + the exit code the CLI
 * subcommand should propagate.
 *
 * Exit code precedence:
 *   - 64 if CLI is missing or its version is outside [MIN, MAX)
 *   - 65 if CLI is fine but auth is missing/expired/misconfigured
 *   - 0  if all green
 *
 * Version failure outranks auth failure — a version-skewed CLI cannot be
 * trusted to report auth correctly.
 */
export function runPreflight(runner: CommandRunner = defaultRunner): {
  output: PreflightOutput;
  exitCode: number;
} {
  const bin = axhubBin();
  let versionResult: SpawnResult;
  try {
    versionResult = runner([bin, "--version"]);
  } catch {
    // Bun.spawnSync throws when the binary is not found on PATH. Treat as
    // "CLI absent" rather than crashing the helper.
    versionResult = { exitCode: 127, stdout: "", stderr: "binary not found" };
  }

  const cliPresent = versionResult.exitCode === EXIT_OK && versionResult.stdout.length > 0;
  const cliVersion = cliPresent ? extractSemver(versionResult.stdout) : null;

  const inRange =
    cliVersion !== null
    && semver.gte(cliVersion, MIN_AXHUB_CLI_VERSION)
    && semver.lt(cliVersion, MAX_AXHUB_CLI_VERSION);
  const tooOld = cliVersion !== null && semver.lt(cliVersion, MIN_AXHUB_CLI_VERSION);
  const tooNew = cliVersion !== null && semver.gte(cliVersion, MAX_AXHUB_CLI_VERSION);

  // Skip auth call when the CLI is unavailable — saves a guaranteed-failure
  // spawn and keeps preflight under the 50ms hook gate target.
  let authStatus: AuthStatus = { ok: false, code: "cli_unavailable", detail: "" };
  if (cliPresent) {
    let authResult: SpawnResult;
    try {
      authResult = runner([bin, "auth", "status", "--json"]);
    } catch {
      authResult = { exitCode: 1, stdout: "", stderr: "auth status spawn failed" };
    }
    authStatus = parseAuthStatus(authResult.stdout);
  }

  const cache = readLastDeployCache();
  const output: PreflightOutput = {
    cli_version: cliVersion,
    in_range: inRange,
    cli_too_old: tooOld,
    cli_too_new: tooNew,
    cli_present: cliPresent,
    auth_ok: authStatus.ok,
    auth_error_code: authStatus.ok ? null : authStatus.code,
    scopes: authStatus.ok ? authStatus.scopes : [],
    profile: process.env["AXHUB_PROFILE"] || null,
    endpoint: process.env["AXHUB_ENDPOINT"] || null,
    user_email: authStatus.ok ? authStatus.user_email : null,
    expires_at: authStatus.ok ? authStatus.expires_at : null,
    current_app: process.env["AXHUB_APP_SLUG"] || cache?.app_slug || null,
    current_env: process.env["AXHUB_PROFILE"] || null,
    last_deploy_id: cache?.deployment_id || null,
    last_deploy_status: cache?.status || null,
    plugin_version: process.env["AXHUB_PLUGIN_VERSION"] || "0.1.17",
  };

  if (!cliPresent || !inRange) return { output, exitCode: EXIT_USAGE };
  if (!authStatus.ok) return { output, exitCode: EXIT_AUTH };
  return { output, exitCode: EXIT_OK };
}
