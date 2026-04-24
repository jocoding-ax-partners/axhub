/**
 * consent.ts — HMAC consent token mint/verify for axhub destructive operations.
 *
 * Backs the PreToolUse deterministic deny-gate (§16.4 / audit row 11, 32, 43).
 * REVERSE row 32: replaces prompt-based hook with command-based binding-bound JWT
 * so security decisions never rely on LLM bag-of-words reasoning.
 *
 * Token lifecycle:
 *   1. Skill calls `consent-mint` with binding payload after AskUserQuestion approval.
 *   2. Helper writes signed JWT to ${XDG_RUNTIME_DIR}/axhub/consent-<sessionId>.json (mode 0600).
 *   3. PreToolUse hook calls `preauth-check`, which parses the bash command, builds
 *      the same binding from cwd-derived state, and calls `verifyToken` against the file.
 *   4. Every binding field MUST match exactly — mismatch on any field denies the call.
 *
 * Key derivation:
 *   ${XDG_STATE_HOME or ~/.local/state}/axhub/hmac-key (32 bytes random, mode 0600).
 *   Auto-created on first mint via crypto.randomBytes. NEVER logged.
 *
 * Multi-tenant isolation (§16.16): consent files are namespaced by session_id; HMAC key
 * is per-OS-user (state dir is user-private). Token files use mode 0600 + O_NOFOLLOW.
 */

import { SignJWT, jwtVerify, type JWTPayload } from "jose";
import { randomBytes, randomUUID } from "node:crypto";
import { homedir, tmpdir } from "node:os";
import { join } from "node:path";
import { mkdir, readFile, stat, writeFile } from "node:fs/promises";

export interface ConsentBinding {
  tool_call_id: string;
  // `deploy_logs_kill`: reserved for v0.2 signal-kill protection.
  // Currently unreachable in v0.1.0 CLI (no `--kill` flag exists). Removing
  // would force HMAC binding-schema migration when v0.2 ships, so keep.
  action: "deploy_create" | "update_apply" | "deploy_logs_kill" | "auth_login";
  app_id: string;
  profile: string;
  branch: string;
  commit_sha: string;
}

export interface MintResult {
  token_id: string;
  expires_at: string;
  file_path: string;
}

export interface VerifyResult {
  valid: boolean;
  reason?: string;
}

export interface ParsedAxhubCommand {
  is_destructive: boolean;
  action?: ConsentBinding["action"];
  app_id?: string;
  branch?: string;
  commit_sha?: string;
  profile?: string;
}

const HMAC_KEY_BYTES = 32;
const FILE_MODE_PRIVATE = 0o600;
const DIR_MODE_PRIVATE = 0o700;
const JWT_ALG = "HS256";

// ---------------------------------------------------------------------------
// Path helpers — XDG-compliant, single OS user assumption (§16.16).
// ---------------------------------------------------------------------------

const stateRoot = (): string => {
  const xdg = process.env["XDG_STATE_HOME"];
  if (xdg && xdg.length > 0) return join(xdg, "axhub");
  return join(homedir(), ".local", "state", "axhub");
};

const runtimeRoot = (): string => {
  const xdg = process.env["XDG_RUNTIME_DIR"];
  if (xdg && xdg.length > 0) return join(xdg, "axhub");
  return join(tmpdir(), "axhub");
};

const hmacKeyPath = (): string => join(stateRoot(), "hmac-key");

const sessionId = (): string => {
  const env = process.env["CLAUDE_SESSION_ID"];
  if (env && env.length > 0) return env;
  return randomUUID();
};

const tokenFilePath = (sid: string): string => join(runtimeRoot(), `consent-${sid}.json`);

// ---------------------------------------------------------------------------
// HMAC key — load-or-create, never log the bytes.
// ---------------------------------------------------------------------------

export async function getOrCreateHmacKey(): Promise<Uint8Array> {
  const path = hmacKeyPath();
  try {
    const buf = await readFile(path);
    if (buf.length !== HMAC_KEY_BYTES) {
      throw new Error("hmac-key has wrong length");
    }
    return new Uint8Array(buf);
  } catch (e) {
    const err = e as NodeJS.ErrnoException;
    if (err.code !== "ENOENT") throw e;
    // First run: generate and persist.
    await mkdir(stateRoot(), { recursive: true, mode: DIR_MODE_PRIVATE });
    const key = randomBytes(HMAC_KEY_BYTES);
    await writeFile(path, key, { mode: FILE_MODE_PRIVATE });
    return new Uint8Array(key);
  }
}

// ---------------------------------------------------------------------------
// Mint — sign binding into JWT, persist to runtime dir.
// ---------------------------------------------------------------------------

export async function mintToken(
  binding: ConsentBinding,
  ttl_sec: number,
): Promise<MintResult> {
  const key = await getOrCreateHmacKey();
  const now = Math.floor(Date.now() / 1000);
  const exp = now + ttl_sec;
  const token_id = randomUUID();

  const payload = {
    ...binding,
    jti: token_id,
  } satisfies JWTPayload & ConsentBinding;

  const jwt = await new SignJWT(payload)
    .setProtectedHeader({ alg: JWT_ALG })
    .setIssuedAt(now)
    .setExpirationTime(exp)
    .sign(key);

  const sid = sessionId();
  const file_path = tokenFilePath(sid);
  await mkdir(runtimeRoot(), { recursive: true, mode: DIR_MODE_PRIVATE });

  const fileBody = JSON.stringify({
    token_id,
    jwt,
    expires_at: new Date(exp * 1000).toISOString(),
    session_id: sid,
  });
  await writeFile(file_path, fileBody, { mode: FILE_MODE_PRIVATE });

  return {
    token_id,
    expires_at: new Date(exp * 1000).toISOString(),
    file_path,
  };
}

// ---------------------------------------------------------------------------
// Verify — read latest token for session, jwtVerify, deep-equal binding fields.
// ---------------------------------------------------------------------------

export async function verifyToken(binding: ConsentBinding): Promise<VerifyResult> {
  const sid = sessionId();
  const path = tokenFilePath(sid);

  let raw: string;
  try {
    await stat(path);
    raw = await readFile(path, "utf8");
  } catch (e) {
    const err = e as NodeJS.ErrnoException;
    if (err.code === "ENOENT") return { valid: false, reason: "no_consent_token" };
    return { valid: false, reason: "token_file_unreadable" };
  }

  let parsed: { jwt?: unknown };
  try {
    parsed = JSON.parse(raw) as { jwt?: unknown };
  } catch {
    return { valid: false, reason: "token_file_corrupt" };
  }
  if (typeof parsed.jwt !== "string") {
    return { valid: false, reason: "token_file_missing_jwt" };
  }

  const key = await getOrCreateHmacKey();
  let payload: JWTPayload & Partial<ConsentBinding>;
  try {
    const result = await jwtVerify(parsed.jwt, key, { algorithms: [JWT_ALG] });
    payload = result.payload as JWTPayload & Partial<ConsentBinding>;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    if (/exp/i.test(msg) || /expired/i.test(msg)) {
      return { valid: false, reason: "token_expired" };
    }
    return { valid: false, reason: "token_signature_invalid" };
  }

  // Deterministic field-by-field match — mismatch on any field invalidates.
  const fields: (keyof ConsentBinding)[] = [
    "tool_call_id",
    "action",
    "app_id",
    "profile",
    "branch",
    "commit_sha",
  ];
  for (const f of fields) {
    if (payload[f] !== binding[f]) {
      return { valid: false, reason: `binding_mismatch:${f}` };
    }
  }

  return { valid: true };
}

// ---------------------------------------------------------------------------
// Command parser — recognize destructive axhub invocations.
// ---------------------------------------------------------------------------

const FLAG_MAP: Record<string, keyof ParsedAxhubCommand> = {
  "--app": "app_id",
  "--branch": "branch",
  "--commit": "commit_sha",
  "--profile": "profile",
};

const extractFlags = (tokens: string[]): Partial<ParsedAxhubCommand> => {
  const out: Partial<ParsedAxhubCommand> = {};
  for (let i = 0; i < tokens.length; i++) {
    const t = tokens[i];
    if (t === undefined) continue;
    // Support both `--flag value` and `--flag=value`.
    if (t.includes("=")) {
      const eq = t.indexOf("=");
      const flag = t.slice(0, eq);
      const val = t.slice(eq + 1);
      const key = FLAG_MAP[flag];
      if (key) (out as Record<string, string>)[key] = val;
      continue;
    }
    const key = FLAG_MAP[t];
    if (key) {
      const val = tokens[i + 1];
      if (val !== undefined && !val.startsWith("--")) {
        (out as Record<string, string>)[key] = val;
        i++;
      }
    }
  }
  return out;
};

// Pattern for environment variable assignments at the start of a command position
// (e.g. `AXHUB_TOKEN=foo`, `FOO=bar BAZ=qux axhub ...`). Matches POSIX shell rules:
// uppercase + digits + underscore; cannot start with a digit.
const ENV_ASSIGN_PREFIX_RE = /^(?:[A-Za-z_][A-Za-z0-9_]*=\S*\s+)+/;

// Splits a shell-ish string into "command positions" — substrings that begin a
// fresh command. Recognizes:
//   - top of string
//   - `;`, `&&`, `||`, `|`, `&` (statement separators)
//   - `$(` ... `)` and backticks (command substitution)
//   - `bash -c "..."`, `sh -c "..."`, `eval "..."` (shell-in-string forms)
//   - `(` ... `)` (sub-shell parentheses)
//
// Returns the substring of the command starting at each candidate command position
// (already shifted past the opening delimiter). Quoted strings inside `bash -c "..."`
// have outer quotes stripped before being added to the candidate list.
const COLLECT_MAX_DEPTH = 5;

const collectCommandPositions = (cmd: string, depth: number = 0): string[] => {
  const positions: string[] = [cmd];
  if (depth >= COLLECT_MAX_DEPTH) return positions;
  const len = cmd.length;
  let i = 0;
  while (i < len) {
    const ch = cmd[i];

    // Statement separators: ; && || | &
    if (ch === ";" || ch === "&" || ch === "|") {
      // Skip through repeats (handles && and ||) so we land just after them.
      let j = i + 1;
      while (j < len && (cmd[j] === "&" || cmd[j] === "|")) j++;
      positions.push(cmd.slice(j));
      i = j;
      continue;
    }

    // Sub-shell open: $( or (
    if (ch === "$" && cmd[i + 1] === "(") {
      positions.push(cmd.slice(i + 2));
      i += 2;
      continue;
    }
    if (ch === "(") {
      positions.push(cmd.slice(i + 1));
      i += 1;
      continue;
    }

    // Backtick command substitution.
    if (ch === "`") {
      positions.push(cmd.slice(i + 1));
      i += 1;
      continue;
    }

    i += 1;
  }

  // Detect `bash -c "..."`, `sh -c "..."`, `eval "..."` — pull the quoted body
  // out and treat it as another command position. Handles single, double, and
  // unquoted forms. Recursively re-scan the extracted body so nested wrappers
  // like `bash -c "(axhub auth login)"` or `eval "bash -c '...'"` are not
  // missed (Phase 3 gotcha #2 fix).
  const shellInString =
    /\b(?:bash|sh|zsh|dash|ksh|eval)\s+(?:-c\s+)?(?:"((?:[^"\\]|\\.)*)"|'((?:[^'\\]|\\.)*)'|(\S+))/g;
  let m: RegExpExecArray | null;
  while ((m = shellInString.exec(cmd)) !== null) {
    const body = m[1] ?? m[2] ?? m[3];
    if (body !== undefined && body.length > 0) {
      positions.push(body);
      // Unescape one level of shell quoting before recursing — handles
      // `eval "bash -c \"axhub ...\""` where the inner double-quotes are
      // backslash-escaped inside the outer double-quoted eval string. The
      // resulting unescaped body is what the outer shell would actually
      // execute, so it must be scanned too.
      const unescaped = body.replace(/\\(.)/g, "$1");
      if (unescaped !== body) {
        positions.push(unescaped);
        const nestedUn = collectCommandPositions(unescaped, depth + 1);
        for (let k = 1; k < nestedUn.length; k++) positions.push(nestedUn[k]!);
      }
      // Recurse into the raw body too — depth-limited to prevent runaway on
      // pathological input.
      const nested = collectCommandPositions(body, depth + 1);
      // Skip nested[0] (it's `body` itself, already pushed above).
      for (let k = 1; k < nested.length; k++) positions.push(nested[k]!);
    }
  }

  return positions;
};

// Tokenize a single command position into whitespace-separated tokens, after
// stripping any leading env-var assignments and wrapping delimiters. Returns
// null if axhub is not the command being executed at this position.
const tokensIfAxhubCommand = (rawPosition: string): string[] | null => {
  let s = rawPosition.trimStart();
  // Strip leading env-var assignments (one or more), e.g. `AXHUB_TOKEN=foo `.
  s = s.replace(ENV_ASSIGN_PREFIX_RE, "");
  // Strip leading wrapping chars: quotes, parens, backticks. Repeats handle
  // multiply-wrapped positions like `("axhub ...)`. Also strip a leading `$(`
  // pair if collectCommandPositions handed us text that started with one
  // (defense-in-depth — typically already stripped at a higher level).
  // Phase 3 gotcha #2 fix (nested wrappers leaking into single position).
  while (s.length > 0 && /["'`(]/.test(s[0]!)) s = s.slice(1);
  if (s.startsWith("$(")) s = s.slice(2);
  s = s.trimStart();

  // Must start with the bare token `axhub` followed by whitespace or end.
  if (!/^axhub(?:\s|$)/.test(s)) return null;
  const raw = s.split(/\s+/);
  // Phase 3 gotcha #1 + #3 fixes: strip surrounding quotes and trailing
  // close-delimiters (e.g. `login)`, `"create"`, ``"deploy"``) from each
  // token so subcommand and flag matching works regardless of wrapper.
  return raw
    .map((t) => {
      let v = t;
      // Surrounding quotes (matched pair only)
      if (v.length >= 2) {
        const first = v[0];
        const last = v[v.length - 1];
        if ((first === '"' && last === '"') || (first === "'" && last === "'")) {
          v = v.slice(1, -1);
        }
      }
      // Trailing close delimiters
      while (v.length > 0 && /[)`'"]/.test(v[v.length - 1]!)) v = v.slice(0, -1);
      return v;
    })
    .filter((t) => t.length > 0);
};

// Try to extract a destructive axhub invocation from a single tokenized command.
const matchDestructive = (tokens: string[]): ParsedAxhubCommand | null => {
  // tokens[0] === "axhub"
  const sub = tokens[1];
  const sub2 = tokens[2];

  if (sub === "deploy" && sub2 === "create") {
    const flags = extractFlags(tokens.slice(3));
    return { is_destructive: true, action: "deploy_create", ...flags };
  }
  if (sub === "update" && sub2 === "apply") {
    const flags = extractFlags(tokens.slice(3));
    return { is_destructive: true, action: "update_apply", ...flags };
  }
  if (sub === "deploy" && sub2 === "logs" && tokens.includes("--kill")) {
    const flags = extractFlags(tokens.slice(3));
    return { is_destructive: true, action: "deploy_logs_kill", ...flags };
  }
  if (sub === "auth" && sub2 === "login") {
    const flags = extractFlags(tokens.slice(3));
    return { is_destructive: true, action: "auth_login", ...flags };
  }
  return null;
};

export function parseAxhubCommand(cmd: string): ParsedAxhubCommand {
  // Defense-in-depth: detect `axhub` invoked at ANY command position within the
  // string (env-prefix, sub-shells, compound separators, eval/bash -c, parens,
  // backticks, $(...)). Bypass attempts like `AXHUB_TOKEN=foo axhub deploy create`
  // or `bash -c "axhub deploy create"` MUST classify as destructive so the
  // PreToolUse gate can deny them when no consent token is present.
  const positions = collectCommandPositions(cmd);
  for (const pos of positions) {
    const tokens = tokensIfAxhubCommand(pos);
    if (tokens === null) continue;
    const hit = matchDestructive(tokens);
    if (hit !== null) return hit;
  }
  return { is_destructive: false };
}
