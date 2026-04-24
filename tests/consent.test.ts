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
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { randomUUID } from "node:crypto";
import {
  mintToken,
  parseAxhubCommand,
  verifyToken,
  type ConsentBinding,
} from "../src/axhub-helpers/consent.ts";

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

  test("expired token: ttl=1, sleep 2s, verify fails with expired reason", async () => {
    const binding = baseBinding();
    await mintToken(binding, 1);
    await sleep(2100);
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
