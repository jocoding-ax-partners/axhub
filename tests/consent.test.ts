/**
 * tests/consent.test.ts — HMAC consent token roundtrip + binding enforcement.
 *
 * Validates US-002 deliverables:
 *   1. mint+verify roundtrip
 *   2. expired token (ttl=1, sleep 2)
 *   3. wrong app_id mismatch
 *   4. wrong profile mismatch
 *   5. missing token file
 *   6. parseAxhubCommand classification
 *
 * HMAC key MUST NOT appear in logs/stderr (caller greps run output).
 *
 * Each test uses a unique CLAUDE_SESSION_ID + isolated XDG dirs so concurrent
 * runs don't collide and we don't pollute the developer's real state dir.
 */

import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { randomUUID } from "node:crypto";
import {
  mintToken,
  parseAxhubCommand,
  verifyToken,
  type ConsentBinding,
} from "../src/axhub-helpers/consent.ts";

const REPO_ROOT = join(import.meta.dir, "..");

const sleep = (ms: number): Promise<void> =>
  new Promise((resolve) => setTimeout(resolve, ms));

const baseBinding = (overrides: Partial<ConsentBinding> = {}): ConsentBinding => ({
  tool_call_id: "sess-abc:tc-1",
  action: "deploy_create",
  app_id: "paydrop",
  profile: "prod",
  branch: "main",
  commit_sha: "a3f9c1b",
  ...overrides,
});

let tmpRoot: string;
let prevState: string | undefined;
let prevRuntime: string | undefined;
let prevSession: string | undefined;

beforeEach(() => {
  tmpRoot = mkdtempSync(join(tmpdir(), "axhub-consent-test-"));
  prevState = process.env["XDG_STATE_HOME"];
  prevRuntime = process.env["XDG_RUNTIME_DIR"];
  prevSession = process.env["CLAUDE_SESSION_ID"];
  process.env["XDG_STATE_HOME"] = join(tmpRoot, "state");
  process.env["XDG_RUNTIME_DIR"] = join(tmpRoot, "runtime");
  process.env["CLAUDE_SESSION_ID"] = `test-${randomUUID()}`;
});

afterEach(() => {
  if (prevState === undefined) delete process.env["XDG_STATE_HOME"];
  else process.env["XDG_STATE_HOME"] = prevState;
  if (prevRuntime === undefined) delete process.env["XDG_RUNTIME_DIR"];
  else process.env["XDG_RUNTIME_DIR"] = prevRuntime;
  if (prevSession === undefined) delete process.env["CLAUDE_SESSION_ID"];
  else process.env["CLAUDE_SESSION_ID"] = prevSession;
  rmSync(tmpRoot, { recursive: true, force: true });
});

describe("mintToken / verifyToken", () => {
  test("roundtrip: matching binding verifies as valid", async () => {
    const binding = baseBinding();
    const minted = await mintToken(binding, 60);
    expect(minted.token_id).toMatch(/^[0-9a-f-]{36}$/i);
    expect(minted.expires_at).toMatch(/^\d{4}-\d{2}-\d{2}T/);
    expect(minted.file_path).toContain("consent-");

    const result = await verifyToken(binding);
    expect(result.valid).toBe(true);
    expect(result.reason).toBeUndefined();
  });

  test("mint fails fast without CLAUDE_SESSION_ID instead of writing an unverifiable token", async () => {
    delete process.env["CLAUDE_SESSION_ID"];

    await expect(mintToken(baseBinding(), 60)).rejects.toThrow(/CLAUDE_SESSION_ID/);
    expect(existsSync(join(tmpRoot, "state", "axhub", "hmac-key"))).toBe(false);
    expect(existsSync(join(tmpRoot, "runtime", "axhub"))).toBe(false);
  });

  test("CLI consent-mint also fails fast without CLAUDE_SESSION_ID across process boundary", () => {
    const env = { ...process.env };
    delete env["CLAUDE_SESSION_ID"];
    env["XDG_STATE_HOME"] = join(tmpRoot, "state");
    env["XDG_RUNTIME_DIR"] = join(tmpRoot, "runtime");

    const result = spawnSync("bun", ["src/axhub-helpers/index.ts", "consent-mint"], {
      cwd: REPO_ROOT,
      env,
      input: JSON.stringify(baseBinding()),
      encoding: "utf8",
      timeout: 10000,
    });

    expect(result.status).not.toBe(0);
    expect(result.stderr).toContain("CLAUDE_SESSION_ID");
  });

  test.skipIf(process.platform === "win32")(
    "mint rejects a symlinked consent file instead of overwriting its target",
    async () => {
      const runtimeRoot = join(tmpRoot, "runtime", "axhub");
      mkdirSync(runtimeRoot, { recursive: true });
      const targetPath = join(tmpRoot, "symlink-target.json");
      const originalTarget = "do-not-overwrite";
      writeFileSync(targetPath, originalTarget);
      symlinkSync(targetPath, join(runtimeRoot, `consent-${process.env["CLAUDE_SESSION_ID"]}.json`));

      await expect(mintToken(baseBinding(), 60)).rejects.toThrow(/symlink|consent/i);
      expect(readFileSync(targetPath, "utf8")).toBe(originalTarget);
    },
  );

  test("expired token: ttl=1, sleep 2s, verify fails with expired reason", async () => {
    const binding = baseBinding();
    await mintToken(binding, 1);
    await sleep(2100);
    const result = await verifyToken(binding);
    expect(result.valid).toBe(false);
    expect(result.reason).toBeDefined();
    expect(result.reason!.toLowerCase()).toContain("expired");
  });

  test("zero leeway: exp = now - 1 is rejected", async () => {
    const binding = baseBinding();
    await mintToken(binding, -1);
    const result = await verifyToken(binding);
    expect(result.valid).toBe(false);
    expect(result.reason).toBeDefined();
    expect(result.reason!.toLowerCase()).toContain("expired");
  });

  test("zero leeway: exp = now boundary is rejected", async () => {
    const binding = baseBinding();
    await mintToken(binding, 0);
    const result = await verifyToken(binding);
    expect(result.valid).toBe(false);
    expect(result.reason).toBeDefined();
    expect(result.reason!.toLowerCase()).toContain("expired");
  });

  test("wrong app_id: minted with paydrop, verified with otherapp → invalid", async () => {
    await mintToken(baseBinding({ app_id: "paydrop" }), 60);
    const result = await verifyToken(baseBinding({ app_id: "otherapp" }));
    expect(result.valid).toBe(false);
    expect(result.reason).toBe("binding_mismatch:app_id");
  });

  test("wrong profile: minted with prod, verified with staging → invalid", async () => {
    await mintToken(baseBinding({ profile: "prod" }), 60);
    const result = await verifyToken(baseBinding({ profile: "staging" }));
    expect(result.valid).toBe(false);
    expect(result.reason).toBe("binding_mismatch:profile");
  });

  test("missing token file: verify with no prior mint → invalid no_consent_token", async () => {
    const result = await verifyToken(baseBinding());
    expect(result.valid).toBe(false);
    expect(result.reason).toBe("no_consent_token");
  });

  test("wrong tool_call_id: rejects token from different call", async () => {
    await mintToken(baseBinding({ tool_call_id: "sess-abc:tc-1" }), 60);
    const result = await verifyToken(baseBinding({ tool_call_id: "sess-abc:tc-2" }));
    expect(result.valid).toBe(false);
    expect(result.reason).toBe("binding_mismatch:tool_call_id");
  });

  test("wrong commit_sha: rejects after force-push", async () => {
    await mintToken(baseBinding({ commit_sha: "a3f9c1b" }), 60);
    const result = await verifyToken(baseBinding({ commit_sha: "deadbee" }));
    expect(result.valid).toBe(false);
    expect(result.reason).toBe("binding_mismatch:commit_sha");
  });
});

describe("parseAxhubCommand", () => {
  test("axhub deploy create with --app/--branch/--commit extracts all flags", () => {
    const r = parseAxhubCommand(
      "axhub deploy create --app paydrop --branch main --commit a3f9c1b --json",
    );
    expect(r.is_destructive).toBe(true);
    expect(r.action).toBe("deploy_create");
    expect(r.app_id).toBe("paydrop");
    expect(r.branch).toBe("main");
    expect(r.commit_sha).toBe("a3f9c1b");
  });

  test("axhub deploy create with --profile flag", () => {
    const r = parseAxhubCommand(
      "axhub deploy create --app foo --profile staging --branch dev --commit abc1234",
    );
    expect(r.is_destructive).toBe(true);
    expect(r.profile).toBe("staging");
    expect(r.app_id).toBe("foo");
  });

  test("axhub update apply --force is destructive update_apply", () => {
    const r = parseAxhubCommand("axhub update apply --force");
    expect(r.is_destructive).toBe(true);
    expect(r.action).toBe("update_apply");
  });

  test("axhub auth login is destructive auth_login", () => {
    const r = parseAxhubCommand("axhub auth login");
    expect(r.is_destructive).toBe(true);
    expect(r.action).toBe("auth_login");
  });

  test("axhub deploy logs --follow --kill is destructive deploy_logs_kill", () => {
    const r = parseAxhubCommand("axhub deploy logs --follow --kill --app foo");
    expect(r.is_destructive).toBe(true);
    expect(r.action).toBe("deploy_logs_kill");
    expect(r.app_id).toBe("foo");
  });

  test("ls -la is not destructive (not even axhub)", () => {
    const r = parseAxhubCommand("ls -la");
    expect(r.is_destructive).toBe(false);
    expect(r.action).toBeUndefined();
  });

  test("axhub status is not destructive (read-only)", () => {
    const r = parseAxhubCommand("axhub status");
    expect(r.is_destructive).toBe(false);
  });

  test("axhub deploy logs without --kill is not destructive", () => {
    const r = parseAxhubCommand("axhub deploy logs --follow --app foo");
    expect(r.is_destructive).toBe(false);
  });

  test("supports --flag=value form", () => {
    const r = parseAxhubCommand(
      "axhub deploy create --app=paydrop --branch=main --commit=abc",
    );
    expect(r.is_destructive).toBe(true);
    expect(r.app_id).toBe("paydrop");
    expect(r.branch).toBe("main");
    expect(r.commit_sha).toBe("abc");
  });
});

describe("parseAxhubCommand — bypass hardening (T-ADV-PARSE-1..8)", () => {
  // Defense-in-depth: parser must not be fooled by env-var prefixes, sub-shells,
  // compound operators, eval/bash -c, or paren-wrapped invocations.
  test("T-ADV-PARSE-1: env-var prefix `AXHUB_TOKEN=foo axhub deploy create` is destructive", () => {
    const r = parseAxhubCommand(
      "AXHUB_TOKEN=foo axhub deploy create --app paydrop --branch main --commit abc",
    );
    expect(r.is_destructive).toBe(true);
    expect(r.action).toBe("deploy_create");
    expect(r.app_id).toBe("paydrop");
  });

  test("T-ADV-PARSE-2: bash -c sub-shell `bash -c \"axhub deploy create ...\"` is destructive", () => {
    const r = parseAxhubCommand(
      'bash -c "axhub deploy create --app paydrop --branch main --commit abc"',
    );
    expect(r.is_destructive).toBe(true);
    expect(r.action).toBe("deploy_create");
    expect(r.app_id).toBe("paydrop");
  });

  test("T-ADV-PARSE-3: compound `cd /tmp && axhub deploy create` is destructive", () => {
    const r = parseAxhubCommand(
      "cd /tmp && axhub deploy create --app paydrop --branch main --commit abc",
    );
    expect(r.is_destructive).toBe(true);
    expect(r.action).toBe("deploy_create");
    expect(r.app_id).toBe("paydrop");
  });

  test("T-ADV-PARSE-4: leading `; axhub deploy create` is destructive", () => {
    const r = parseAxhubCommand(
      "; axhub deploy create --app paydrop --branch main --commit abc",
    );
    expect(r.is_destructive).toBe(true);
    expect(r.action).toBe("deploy_create");
    expect(r.app_id).toBe("paydrop");
  });

  test("T-ADV-PARSE-5: eval-quoted `eval \"axhub deploy create ...\"` is destructive", () => {
    const r = parseAxhubCommand(
      'eval "axhub deploy create --app paydrop --branch main --commit abc"',
    );
    expect(r.is_destructive).toBe(true);
    expect(r.action).toBe("deploy_create");
    expect(r.app_id).toBe("paydrop");
  });

  test("T-ADV-PARSE-6: paren sub-shell `(axhub deploy create ...)` is destructive", () => {
    const r = parseAxhubCommand(
      "(axhub deploy create --app paydrop --branch main --commit abc)",
    );
    expect(r.is_destructive).toBe(true);
    expect(r.action).toBe("deploy_create");
    expect(r.app_id).toBe("paydrop");
  });

  test("T-ADV-PARSE-7: `echo axhub deploy create` is NOT destructive (axhub is an argument)", () => {
    const r = parseAxhubCommand("echo axhub deploy create");
    expect(r.is_destructive).toBe(false);
    expect(r.action).toBeUndefined();
  });

  test("T-ADV-PARSE-8: `axhub apps list --json` (read-only) remains NOT destructive", () => {
    const r = parseAxhubCommand("axhub apps list --json");
    expect(r.is_destructive).toBe(false);
    expect(r.action).toBeUndefined();
  });
});

describe("parseAxhubCommand — deploy_logs_kill is unreachable in v0.1.0", () => {
  // CLI v0.1.0 has no --kill flag. parseAxhubCommand should NEVER
  // produce action: "deploy_logs_kill" for any valid v0.1.0 command surface.
  // If this test ever fails, v0.2 has shipped --kill (or similar) and the
  // gate must be re-implemented properly.
  const v01Commands = [
    "axhub apps list --json",
    "axhub apps list --json --per-page=10",
    "axhub apis list --json --query auth",
    "axhub apis list --app-id 42 --json",
    "axhub auth status --json",
    "axhub auth login",
    "axhub auth logout",
    "axhub deploy create --app paydrop --branch main --commit abc123",
    "axhub deploy create --app paydrop --branch main --commit abc123 --json",
    "axhub deploy status dep_42 --watch --json",
    "axhub deploy logs dep_42 --follow --source build --json",
    "axhub deploy logs dep_42 --source build",
    "axhub update check --json",
    "axhub update apply --yes",
    "axhub update apply --force --yes",
    "axhub --version",
    "axhub --help",
  ];

  for (const cmd of v01Commands) {
    test(`v0.1.0 command never produces deploy_logs_kill: ${cmd.slice(0, 60)}`, () => {
      const result = parseAxhubCommand(cmd);
      expect(result.action).not.toBe("deploy_logs_kill");
    });
  }
});

// Phase 3 US-201: parser gotchas surfaced by Phase 2 fuzzer iteration.
// These were intentionally avoided by the fuzzer ("stress, not gotchas") and
// are now closed by trailing-delimiter strip + recursive shellInString +
// surrounding-quote strip in tokensIfAxhubCommand.

describe("parseAxhubCommand — Gotcha #1: trailing close-delimiter on action token", () => {
  test("(axhub auth login) — paren-wrapped detected as auth_login", () => {
    const r = parseAxhubCommand("(axhub auth login)");
    expect(r.is_destructive).toBe(true);
    expect(r.action).toBe("auth_login");
  });

  test("`axhub auth login` — backtick-wrapped detected as auth_login", () => {
    const r = parseAxhubCommand("`axhub auth login`");
    expect(r.is_destructive).toBe(true);
    expect(r.action).toBe("auth_login");
  });

  test("(axhub deploy create --app paydrop --branch main --commit abc) — paren-wrapped deploy detected with extracted flags", () => {
    const r = parseAxhubCommand("(axhub deploy create --app paydrop --branch main --commit abc)");
    expect(r.is_destructive).toBe(true);
    expect(r.action).toBe("deploy_create");
    expect(r.app_id).toBe("paydrop");
    expect(r.branch).toBe("main");
    expect(r.commit_sha).toBe("abc");
  });

  test("flag value with trailing close-delimiter is stripped (--commit abc) → abc)", () => {
    const r = parseAxhubCommand("(axhub deploy create --app paydrop --branch main --commit abc)");
    expect(r.commit_sha).toBe("abc");
    expect(r.commit_sha).not.toBe("abc)");
  });

  test("$(axhub deploy create --app paydrop --branch main --commit abc) — sub-shell wrapper detected", () => {
    const r = parseAxhubCommand("$(axhub deploy create --app paydrop --branch main --commit abc)");
    expect(r.is_destructive).toBe(true);
    expect(r.action).toBe("deploy_create");
    expect(r.app_id).toBe("paydrop");
  });
});

describe("parseAxhubCommand — Gotcha #2: nested sub-shell inside eval/bash -c", () => {
  test('eval "bash -c \\"axhub deploy create --app paydrop --branch main --commit abc\\"" — eval-of-bash detected', () => {
    const r = parseAxhubCommand('eval "bash -c \\"axhub deploy create --app paydrop --branch main --commit abc\\""');
    expect(r.is_destructive).toBe(true);
    expect(r.action).toBe("deploy_create");
  });

  test('bash -c "(axhub auth login)" — bash-c with paren-wrapped inner detected', () => {
    const r = parseAxhubCommand('bash -c "(axhub auth login)"');
    expect(r.is_destructive).toBe(true);
    expect(r.action).toBe("auth_login");
  });

  test('sh -c "$(axhub deploy create --app paydrop --branch main --commit abc)" — sh-c with $() inner detected', () => {
    const r = parseAxhubCommand('sh -c "$(axhub deploy create --app paydrop --branch main --commit abc)"');
    expect(r.is_destructive).toBe(true);
    expect(r.action).toBe("deploy_create");
  });

  test("zsh -c '`axhub auth login`' — zsh-c with backtick inner detected", () => {
    const r = parseAxhubCommand("zsh -c '`axhub auth login`'");
    expect(r.is_destructive).toBe(true);
    expect(r.action).toBe("auth_login");
  });

  test('eval "(bash -c \\"axhub update apply --yes\\")" — triple-nested still detected', () => {
    const r = parseAxhubCommand('eval "(bash -c \\"axhub update apply --yes\\")"');
    expect(r.is_destructive).toBe(true);
    expect(r.action).toBe("update_apply");
  });
});

describe("parseAxhubCommand — read-only allowlist (Phase 5 US-504)", () => {
  // Explicit assertions that read-only commands NEVER classify as destructive.
  // Defensive coverage for the user-trace bug where /axhub:status hit hook deny.

  test("axhub deploy status dep_X --watch --json → not destructive", () => {
    const r = parseAxhubCommand("axhub deploy status dep_42 --app paydrop --watch --json");
    expect(r.is_destructive).toBe(false);
    expect(r.action).toBeUndefined();
  });

  test("axhub deploy logs dep_X --follow --source build → not destructive", () => {
    const r = parseAxhubCommand("axhub deploy logs dep_42 --app paydrop --follow --source build --json");
    expect(r.is_destructive).toBe(false);
    expect(r.action).toBeUndefined();
  });

  test("axhub deploy logs dep_X --source pod → not destructive (no --kill)", () => {
    const r = parseAxhubCommand("axhub deploy logs dep_42 --source pod --json");
    expect(r.is_destructive).toBe(false);
  });

  test("axhub apps list --json → not destructive", () => {
    const r = parseAxhubCommand("axhub apps list --json");
    expect(r.is_destructive).toBe(false);
  });

  test("axhub apis list --json --query auth → not destructive", () => {
    const r = parseAxhubCommand("axhub apis list --json --query auth");
    expect(r.is_destructive).toBe(false);
  });

  test("axhub auth status --json → not destructive (only auth login is)", () => {
    const r = parseAxhubCommand("axhub auth status --json");
    expect(r.is_destructive).toBe(false);
    expect(r.action).toBeUndefined();
  });

  test("axhub --version → not destructive", () => {
    const r = parseAxhubCommand("axhub --version");
    expect(r.is_destructive).toBe(false);
  });

  test("axhub --help → not destructive", () => {
    const r = parseAxhubCommand("axhub --help");
    expect(r.is_destructive).toBe(false);
  });
});

describe("parseAxhubCommand — Gotcha #3: quoted subcommand tokens", () => {
  test('axhub "deploy" "create" --app paydrop — double-quoted subcommands detected', () => {
    const r = parseAxhubCommand('axhub "deploy" "create" --app paydrop --branch main --commit abc');
    expect(r.is_destructive).toBe(true);
    expect(r.action).toBe("deploy_create");
  });

  test("axhub 'auth' 'login' — single-quoted subcommands detected", () => {
    const r = parseAxhubCommand("axhub 'auth' 'login'");
    expect(r.is_destructive).toBe(true);
    expect(r.action).toBe("auth_login");
  });

  test('axhub "update" "apply" --yes — quoted update apply detected', () => {
    const r = parseAxhubCommand('axhub "update" "apply" --yes');
    expect(r.is_destructive).toBe(true);
    expect(r.action).toBe("update_apply");
  });

  test('axhub "deploy" create — mixed quoted/bare subcommands detected', () => {
    const r = parseAxhubCommand('axhub "deploy" create --app paydrop --branch main --commit abc');
    expect(r.is_destructive).toBe(true);
    expect(r.action).toBe("deploy_create");
  });

  test("read-only quoted subcommand stays NOT destructive (axhub 'apps' 'list')", () => {
    const r = parseAxhubCommand("axhub 'apps' 'list' --json");
    expect(r.is_destructive).toBe(false);
    expect(r.action).toBeUndefined();
  });
});
