#!/usr/bin/env bun
/**
 * axhub-helpers — axhub Claude Code plugin adapter binary (TypeScript / Bun runtime).
 *
 * Single multi-command binary that owns: live profile/app resolution, HMAC
 * consent token mint/verify, exit-code Korean classification, output redaction.
 *
 * All skills/commands/hooks invoke axhub-helpers via this binary on PATH
 * (Claude Code adds bin/ to PATH while plugin is enabled).
 *
 * Build: bun run build  (outputs bin/axhub-helpers single binary)
 * Smoke: bun run smoke
 *
 * Subcommands:
 *   session-start    SessionStart hook entry: checks axhub install, version, plugin signature
 *   preauth-check    PreToolUse hook entry: verifies HMAC consent token for destructive ops
 *   consent-mint     Skill entry: mints HMAC consent token after AskUserQuestion approval
 *   consent-verify   Internal: verifies consent token (used by preauth-check)
 *   resolve          Skill entry: live resolves profile/endpoint/app_id/branch/commit
 *   preflight        Skill entry: CLI version range check + auth status pre-flight
 *   classify-exit    PostToolUse hook entry: maps axhub exit code to Korean systemMessage
 *   redact           Filter: NFKC normalize + redact tokens/cross-team URLs from any text
 *
 * All subcommands accept JSON on stdin and emit JSON on stdout (when applicable).
 * All errors go to stderr. Exit codes follow ax-hub-cli convention (0/1/64/65/66/67/68).
 */

import {
  mintToken,
  parseAxhubCommand,
  verifyToken,
  type ConsentBinding,
} from "./consent.ts";
import { classify } from "./catalog.ts";
import { redact } from "./redact.ts";
import { runPreflight } from "./preflight.ts";
import { runResolve } from "./resolve.ts";
import { emitMetaEnvelope } from "./telemetry.ts";
import { runListDeployments } from "./list-deployments.ts";
import { readKeychainToken } from "./keychain.ts";

// CLI I/O primitives: stdout for protocol payloads (JSON to hooks/skills),
// stderr for diagnostics. Avoids console.log to keep this binary's contract
// explicit — every byte on stdout is part of the structured protocol.
const out = (payload: unknown): void => {
  process.stdout.write(typeof payload === "string" ? payload + "\n" : JSON.stringify(payload) + "\n");
};
const outRaw = (text: string): void => {
  process.stdout.write(text);
};
const err = (msg: string): void => {
  process.stderr.write(msg + "\n");
};

// Read all of stdin as utf-8. Bun.stdin.text() is the supported API and also
// works under the compiled binary runtime.
const readStdin = async (): Promise<string> => {
  try {
    return await Bun.stdin.text();
  } catch {
    return "";
  }
};

const parseJson = <T>(raw: string): T | null => {
  if (raw.trim().length === 0) return null;
  try {
    return JSON.parse(raw) as T;
  } catch {
    return null;
  }
};

// Validate a ConsentBinding parsed from JSON input (defensive: stdin is untrusted).
const VALID_ACTIONS: ReadonlySet<ConsentBinding["action"]> = new Set([
  "deploy_create",
  "update_apply",
  // reserved for v0.2 signal-kill protection; currently unreachable in v0.1.0
  "deploy_logs_kill",
  "auth_login",
]);

const asConsentBinding = (v: unknown): ConsentBinding | null => {
  if (v === null || typeof v !== "object") return null;
  const o = v as Record<string, unknown>;
  const strOk = (x: unknown): x is string => typeof x === "string" && x.length > 0;
  if (!strOk(o["tool_call_id"])) return null;
  if (!strOk(o["action"])) return null;
  if (!VALID_ACTIONS.has(o["action"] as ConsentBinding["action"])) return null;
  if (!strOk(o["app_id"])) return null;
  if (!strOk(o["profile"])) return null;
  if (!strOk(o["branch"])) return null;
  if (!strOk(o["commit_sha"])) return null;
  return {
    tool_call_id: o["tool_call_id"],
    action: o["action"] as ConsentBinding["action"],
    app_id: o["app_id"],
    profile: o["profile"],
    branch: o["branch"],
    commit_sha: o["commit_sha"],
  };
};

const PLUGIN_VERSION = "0.1.17";
// MIN_AXHUB_CLI_VERSION + MAX_AXHUB_CLI_VERSION live in ./preflight.ts (the
// only consumer); re-importing here would just create a stale duplicate.
const CONSENT_TOKEN_TTL_SEC = 60;
const HOOK_SCHEMA_VERSION = "v0"; // must match tests/hook-fixtures/v0/

const USAGE = `axhub-helpers - axhub plugin adapter binary (TypeScript / Bun)

Usage:
  axhub-helpers <subcommand> [args]

Subcommands:
  session-start    Hook: SessionStart diagnostics + plugin signature verify
  preauth-check    Hook: PreToolUse HMAC consent gate for destructive axhub ops
  consent-mint     Skill: mint HMAC consent token bound to {action, app, profile, branch, commit}
  consent-verify   Internal: verify consent token
  resolve          Skill: live resolve {profile, endpoint, app_id, app_slug, branch, commit_sha}
  preflight        Skill: CLI version range + auth status check
  classify-exit    Hook: PostToolUse exit code → Korean systemMessage
  redact           Filter: NFKC normalize + redact secrets/cross-team URLs
  list-deployments Skill: GET /api/v1/apps/{id}/deployments — fallback for missing axhub deploy list
  token-import     Skill: read axhub_pat_* from stdin, store at ~/.config/axhub-plugin/token (mode 0600)
  token-init       Skill: 1-step setup — reads token from OS keychain (macOS/Linux) or AXHUB_TOKEN env var
  version          Print version
  help             Show this message`;

async function main(): Promise<number> {
  const [, , cmd, ...args] = process.argv;

  if (!cmd) {
    err(USAGE);
    return 64;
  }

  switch (cmd) {
    case "session-start":
      return cmdSessionStart(args);
    case "preauth-check":
      return cmdPreauthCheck(args);
    case "consent-mint":
      return cmdConsentMint(args);
    case "consent-verify":
      return cmdConsentVerify(args);
    case "resolve":
      return cmdResolve(args);
    case "preflight":
      return cmdPreflight(args);
    case "classify-exit":
      return cmdClassifyExit(args);
    case "redact":
      return cmdRedact(args);
    case "list-deployments":
      return cmdListDeployments(args);
    case "token-import":
      return cmdTokenImport(args);
    case "token-init":
      return cmdTokenInit(args);
    case "version":
    case "--version":
    case "-v":
      out(`axhub-helpers ${PLUGIN_VERSION} (plugin v${PLUGIN_VERSION}, schema ${HOOK_SCHEMA_VERSION})`);
      return 0;
    case "help":
    case "--help":
    case "-h":
      out(USAGE);
      return 0;
    default:
      err(`axhub-helpers: unknown subcommand "${cmd}"\n`);
      err(USAGE);
      return 64;
  }
}

// ============================================================================
// Subcommand stubs (M0 scaffold; M1+ implements full behavior)
// ============================================================================

async function cmdSessionStart(_args: string[]): Promise<number> {
  // TODO M0.5: check axhub binary on PATH, version range (semver compare against
  // MIN_AXHUB_CLI_VERSION/MAX_AXHUB_CLI_VERSION), plugin signature, env hints.
  let systemMessage = `[axhub] M0 scaffold: session-start placeholder. Plugin v${PLUGIN_VERSION} loaded.`;

  // Phase 3 US-204: cosign sidecar advisory (warn, don't block).
  if (process.env["AXHUB_REQUIRE_COSIGN"] === "1") {
    try {
      const { existsSync } = await import("node:fs");
      const selfPath = process.execPath;
      if (selfPath && !existsSync(`${selfPath}.sig`)) {
        systemMessage +=
          "\n\n⚠️ 보안 검증 미통과: 이 helper 바이너리는 cosign 서명이 없어요. 회사 보안 정책에 따라 IT/admin 에 문의해주세요. (계속 사용은 가능해요.)";
      }
    } catch {
      // Best-effort: never let the cosign check break session start.
    }
  }

  out({ systemMessage });
  await emitMetaEnvelope({ event: "session_start" });
  return 0;
}

async function cmdPreauthCheck(_args: string[]): Promise<number> {
  // PreToolUse hook: deterministic deny-gate for destructive axhub bash ops.
  // Early-return allow on: non-Bash tool, non-axhub command, non-destructive axhub.
  // Verify HMAC consent token only for destructive ops (sub-50ms hot path goal).
  const raw = await readStdin();
  const payload = parseJson<{
    session_id?: string;
    tool_call_id?: string;
    tool_name?: string;
    tool_input?: { command?: string };
  }>(raw);

  if (!payload || payload.tool_name !== "Bash") {
    out({ hookSpecificOutput: { hookEventName: "PreToolUse", permissionDecision: "allow" } });
    await emitMetaEnvelope({ event: "preauth_check_allow", reason: "non_bash" });
    return 0;
  }

  const cmd = payload.tool_input?.command ?? "";
  const parsed = parseAxhubCommand(cmd);
  if (!parsed.is_destructive) {
    out({ hookSpecificOutput: { hookEventName: "PreToolUse", permissionDecision: "allow" } });
    await emitMetaEnvelope({ event: "preauth_check_allow", reason: "non_destructive" });
    return 0;
  }

  // Build the same binding the skill minted with. tool_call_id is namespaced by
  // session so that a leaked token from one session can't authorize another.
  const tcid =
    (payload.session_id ?? "") + ":" + (payload.tool_call_id ?? "");
  // auth_login has no app/branch/commit flags — skill mints "_" as placeholder
  // (asConsentBinding requires non-empty strings). Mirror that here so the
  // binding built by preauth-check matches what consent-mint signed.
  const isIdentityAction = parsed.action === "auth_login";
  const binding: ConsentBinding = {
    tool_call_id: tcid,
    action: parsed.action!,
    app_id: parsed.app_id ?? (isIdentityAction ? "_" : ""),
    profile: parsed.profile ?? (isIdentityAction ? (process.env["AXHUB_PROFILE"] ?? "default") : ""),
    branch: parsed.branch ?? (isIdentityAction ? "_" : ""),
    commit_sha: parsed.commit_sha ?? (isIdentityAction ? "_" : ""),
  };

  const result = await verifyToken(binding);
  if (result.valid) {
    out({ hookSpecificOutput: { hookEventName: "PreToolUse", permissionDecision: "allow" } });
    await emitMetaEnvelope({ event: "preauth_check_allow", reason: "consent_verified", action: parsed.action });
    return 0;
  }

  out({
    hookSpecificOutput: { hookEventName: "PreToolUse", permissionDecision: "deny" },
    systemMessage:
      "이 명령은 사전 승인이 필요해요. 먼저 'paydrop 배포해'라고 말해서 승인 카드를 받으세요.",
  });
  await emitMetaEnvelope({ event: "preauth_check_deny", action: parsed.action });
  return 0;
}

async function cmdConsentMint(_args: string[]): Promise<number> {
  // Skill entry: read binding JSON from stdin, mint HMAC token, return location.
  const raw = await readStdin();
  const parsed = parseJson<unknown>(raw);
  const binding = asConsentBinding(parsed);
  if (!binding) {
    err("consent-mint: invalid or missing binding JSON on stdin");
    return 65;
  }
  const result = await mintToken(binding, CONSENT_TOKEN_TTL_SEC);
  out(result);
  await emitMetaEnvelope({ event: "consent_mint", action: binding.action });
  return 0;
}

async function cmdConsentVerify(_args: string[]): Promise<number> {
  // Internal: read binding JSON from stdin, verify, exit 0 if valid else 1.
  const raw = await readStdin();
  const parsed = parseJson<unknown>(raw);
  const binding = asConsentBinding(parsed);
  if (!binding) {
    err("consent-verify: invalid or missing binding JSON on stdin");
    return 65;
  }
  const result = await verifyToken(binding);
  out(result);
  return result.valid ? 0 : 1;
}

async function cmdResolve(args: string[]): Promise<number> {
  // Live resolve {profile, endpoint, app_id, app_slug, branch, commit_sha,
  // commit_message, eta_sec}. Implementation in resolve.ts; this stays a thin
  // adapter so tests drive runResolve() directly with an injected runner.
  const { output, exitCode } = runResolve(args);
  out(output);
  return exitCode;
}

async function cmdPreflight(_args: string[]): Promise<number> {
  // CLI version range gate + auth status. Implementation in preflight.ts;
  // exit code precedence is 64 (version/missing) > 65 (auth) > 0 (ok).
  const { output, exitCode } = runPreflight();
  out(output);
  return exitCode;
}

async function cmdClassifyExit(_args: string[]): Promise<number> {
  // PostToolUse hook: maps axhub exit code to 4-part Korean systemMessage.
  // Early return ({}) on non-axhub Bash commands — 5ms gate.
  const raw = await readStdin();
  const payload = parseJson<{
    tool_input?: { command?: string };
    tool_response?: { exit_code?: number; stdout?: string };
  }>(raw);

  if (!payload) {
    out({});
    return 0;
  }

  const command = payload.tool_input?.command ?? "";
  if (!/^axhub\s/.test(command)) {
    out({});
    return 0;
  }

  const exitCode = payload.tool_response?.exit_code ?? 0;
  const stdout = payload.tool_response?.stdout ?? "";

  // Exit 0: silent for non-deploy commands. Deploy create success deserves a
  // celebration (vibe coder DX), but `axhub --version` / `auth status` /
  // `apps list` exit 0 is just normal completion — emitting "배포 성공" would
  // be confusing and noisy.
  if (exitCode === 0 && !/^axhub\s+deploy\s+create\b/.test(command)) {
    out({});
    return 0;
  }

  const entry = classify(exitCode, stdout);

  let systemMessage = `${entry.emotion}\n\n원인: ${entry.cause}\n\n해결: ${entry.action}`;
  if (entry.button !== undefined) {
    systemMessage += `\n\n선택: ${entry.button}`;
  }

  out({ systemMessage });
  await emitMetaEnvelope({ event: "classify_exit", exit_code: exitCode, command_class: command.split(/\s+/).slice(0, 3).join(" ") });
  return 0;
}

async function cmdRedact(_args: string[]): Promise<number> {
  // Filter: read all stdin, apply NFKC normalize + Bidi/ZWJ strip + secret
  // redaction + ANSI strip. Emit redacted plain text on stdout (no JSON wrap).
  const input = await readStdin();
  outRaw(redact(input));
  return 0;
}

async function cmdListDeployments(args: string[]): Promise<number> {
  // Phase 5 US-501: REST API direct fallback for missing axhub deploy list.
  // Args: --app <id> [--limit <n>]
  let appId = "";
  let limit: number | undefined;
  for (let i = 0; i < args.length; i++) {
    const t = args[i];
    if (t === "--app") appId = args[++i] ?? "";
    else if (t?.startsWith("--app=")) appId = t.slice(6);
    else if (t === "--limit") {
      const n = parseInt(args[++i] ?? "", 10);
      if (Number.isFinite(n)) limit = n;
    } else if (t?.startsWith("--limit=")) {
      const n = parseInt(t.slice(8), 10);
      if (Number.isFinite(n)) limit = n;
    }
  }
  if (appId.length === 0) {
    err("list-deployments: --app <id-or-slug> is required");
    return 64;
  }
  const result = await runListDeployments({ appId, limit });
  out(result);
  return result.exit_code;
}

async function cmdTokenImport(_args: string[]): Promise<number> {
  // Phase 5 US-501: read axhub_pat_* from stdin, store at
  // ${XDG_CONFIG_HOME:-$HOME/.config}/axhub-plugin/token (mode 0600).
  const input = (await readStdin()).trim();
  if (!/^axhub_pat_[A-Za-z0-9_-]{16,}$/.test(input)) {
    err("token-import: stdin does not look like an axhub_pat_* token (expected 'axhub_pat_' + ≥16 chars)");
    return 65;
  }
  const { homedir } = await import("node:os");
  const { join } = await import("node:path");
  const { mkdir, writeFile } = await import("node:fs/promises");
  const xdg = process.env["XDG_CONFIG_HOME"];
  const dir = xdg && xdg.length > 0 ? join(xdg, "axhub-plugin") : join(homedir(), ".config", "axhub-plugin");
  const path = join(dir, "token");
  const oldMask = process.umask(0o077);
  try {
    await mkdir(dir, { recursive: true, mode: 0o700 });
    await writeFile(path, input, { mode: 0o600 });
  } finally {
    process.umask(oldMask);
  }
  out({ stored_at: path, redacted_token: "axhub_pat_[redacted]" });
  return 0;
}

async function cmdTokenInit(_args: string[]): Promise<number> {
  // Token discovery: AXHUB_TOKEN env var → OS keychain (macOS/Linux) → error.

  let token: string;
  let source: string;
  const envToken = process.env["AXHUB_TOKEN"];
  if (envToken !== undefined && envToken.length > 0) {
    token = envToken;
    source = "env-AXHUB_TOKEN";
  } else {
    const result = readKeychainToken();
    if (result.error !== undefined || result.token === undefined) {
      err(`token-init: ${result.error ?? "알 수 없는 에러"}`);
      return 65;
    }
    token = result.token;
    source = result.source ?? "keychain";
  }

  if (token.length < 16) {
    err("token-init: 추출한 token이 너무 짧아요. axhub CLI 재로그인 후 다시 시도해주세요.");
    return 65;
  }

  const { homedir } = await import("node:os");
  const { join } = await import("node:path");
  const { mkdir, writeFile } = await import("node:fs/promises");
  const xdg = process.env["XDG_CONFIG_HOME"];
  const dir =
    xdg && xdg.length > 0 ? join(xdg, "axhub-plugin") : join(homedir(), ".config", "axhub-plugin");
  const path = join(dir, "token");
  const oldMask = process.umask(0o077);
  try {
    await mkdir(dir, { recursive: true, mode: 0o700 });
    await writeFile(path, token, { mode: 0o600 });
  } finally {
    process.umask(oldMask);
  }
  out({
    stored_at: path,
    source,
    redacted_token: token.slice(0, 12) + "...[redacted]",
    next_step: "이제 /axhub:status, /axhub:logs 같은 명령이 자동으로 작동해요.",
  });
  return 0;
}

main()
  .then((code) => process.exit(code))
  .catch((fatal) => {
    err("axhub-helpers: fatal: " + (fatal instanceof Error ? fatal.message : String(fatal)));
    process.exit(1);
  });
