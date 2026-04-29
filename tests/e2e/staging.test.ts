// Phase 3 US-206: ax-hub-cli staging E2E tests.
//
// Real-CLI integration tests gated by env vars. Skip entire suite when
// AXHUB_E2E_STAGING_TOKEN is unset (CI default + local dev default).
//
// To run locally:
//   export AXHUB_E2E_STAGING_TOKEN=<token-from-internal-staging>
//   export AXHUB_E2E_STAGING_ENDPOINT=<staging-api-url>
//   bun run test:e2e
//
// Credential procurement is out-of-scope of this scaffold — see
// docs/RELEASE.md and docs/pilot/admin-rollout.ko.md for how to obtain
// staging access.

import { describe, expect, test, beforeAll } from "bun:test";
import { existsSync } from "node:fs";
import { join } from "node:path";

import { parseAxhubCommand } from "../../src/axhub-helpers/consent";
import { classify } from "../../src/axhub-helpers/catalog";

const E2E_TOKEN = process.env["AXHUB_E2E_STAGING_TOKEN"];
const E2E_ENDPOINT = process.env["AXHUB_E2E_STAGING_ENDPOINT"];
const E2E_APP_ID = process.env["AXHUB_E2E_STAGING_APP_ID"];
const REQUIRE_RUST_HELPER = process.env["AXHUB_E2E_REQUIRE_RUST_HELPER"] === "1";
const RUST_HELPER_PATH = join(import.meta.dir, "../../bin/axhub-helpers");

const E2E_ENABLED = Boolean(E2E_TOKEN && E2E_TOKEN.length > 0);

if (!E2E_ENABLED) {
  process.stderr.write(
    `Skipped: AXHUB_E2E_STAGING_TOKEN not set. See tests/e2e/README.md for how to enable.\n`,
  );
}

const runAxhub = async (args: string[]): Promise<{ exitCode: number; stdout: string; stderr: string }> => {
  const env = { ...process.env };
  if (E2E_TOKEN) env["AXHUB_TOKEN"] = E2E_TOKEN;
  if (E2E_ENDPOINT) env["AXHUB_ENDPOINT"] = E2E_ENDPOINT;
  const proc = Bun.spawnSync({
    cmd: ["axhub", ...args],
    stdout: "pipe",
    stderr: "pipe",
    env,
  });
  return {
    exitCode: proc.exitCode ?? 1,
    stdout: proc.stdout.toString(),
    stderr: proc.stderr.toString(),
  };
};

const expectAppListContract = (apps: unknown[]): void => {
  for (const app of apps) {
    expect(app).toBeTypeOf("object");
    expect(app).not.toBeNull();
    const obj = app as Record<string, unknown>;
    const hasId = typeof obj.id === "string" || typeof obj.id === "number";
    const hasNameOrSlug = typeof obj.name === "string" || typeof obj.slug === "string";
    expect(hasId, `app item missing id: ${JSON.stringify(obj).slice(0, 200)}`).toBe(true);
    expect(hasNameOrSlug, `app item missing name/slug: ${JSON.stringify(obj).slice(0, 200)}`).toBe(true);
  }
};

describe.skipIf(!E2E_ENABLED)("ax-hub-cli staging E2E (gated by AXHUB_E2E_STAGING_TOKEN)", () => {
  beforeAll(() => {
    if (!E2E_ENABLED) return;
    if (!E2E_ENDPOINT) {
      throw new Error("AXHUB_E2E_STAGING_TOKEN set but AXHUB_E2E_STAGING_ENDPOINT not — both required");
    }
  });

  test("axhub auth status --json returns valid identity", async () => {
    const result = await runAxhub(["auth", "status", "--json"]);
    expect(result.exitCode).toBe(0);
    const parsed = JSON.parse(result.stdout) as { user_email?: string; scopes?: string[] };
    expect(parsed.user_email).toBeDefined();
    expect(Array.isArray(parsed.scopes)).toBe(true);
  });

  test("axhub apps list --json returns array (may be empty)", async () => {
    const result = await runAxhub(["apps", "list", "--json"]);
    expect(result.exitCode).toBe(0);
    const parsed = JSON.parse(result.stdout) as unknown;
    // Either bare array or {apps: [...]} — accept both shapes
    if (Array.isArray(parsed)) {
      expectAppListContract(parsed);
    } else if (parsed && typeof parsed === "object" && "apps" in parsed) {
      expect(Array.isArray((parsed as { apps: unknown }).apps)).toBe(true);
      expectAppListContract((parsed as { apps: unknown[] }).apps);
    } else {
      throw new Error(`unexpected apps list shape: ${result.stdout.slice(0, 200)}`);
    }
  });

  test("parseAxhubCommand → action mapping is consistent with real CLI surface", () => {
    // Smoke check: parseAxhubCommand classifications match the real commands
    // we'd run against staging. No actual mutation here — just verifying the
    // parser stays in sync with what the staging CLI accepts as valid syntax.
    type Action = "deploy_create" | "update_apply" | "deploy_logs_kill" | "auth_login";
    const samples: Array<{ cmd: string; destructive: boolean; action?: Action }> = [
      { cmd: "axhub auth status --json", destructive: false },
      { cmd: "axhub apps list --json", destructive: false },
      { cmd: "axhub deploy create --app paydrop --branch main --commit abc", destructive: true, action: "deploy_create" },
      { cmd: "axhub auth login", destructive: true, action: "auth_login" },
    ];
    for (const s of samples) {
      const r = parseAxhubCommand(s.cmd);
      expect(r.is_destructive).toBe(s.destructive);
      if (s.action) expect(r.action).toBe(s.action);
    }
  });

  test("classify-exit produces Korean 4-part template for real exit codes", () => {
    // Pure logic test — does not hit staging. Verifies the catalog covers
    // the exit codes the staging CLI is documented to return.
    for (const exitCode of [0, 1, 64, 65, 66, 67, 68]) {
      const entry = classify(exitCode, "");
      expect(entry.emotion).toBeTypeOf("string");
      expect(entry.cause).toBeTypeOf("string");
      expect(entry.action).toBeTypeOf("string");
      expect(entry.emotion.length).toBeGreaterThan(0);
    }
  });

  test("Rust helper list-deployments hits staging when app id is provided", () => {
    if (!E2E_APP_ID) {
      if (REQUIRE_RUST_HELPER) {
        throw new Error("AXHUB_E2E_REQUIRE_RUST_HELPER=1 requires AXHUB_E2E_STAGING_APP_ID");
      }
      process.stderr.write("Skipped Rust helper staging probe: AXHUB_E2E_STAGING_APP_ID not set.\n");
      return;
    }
    if (!existsSync(RUST_HELPER_PATH)) {
      throw new Error(`Rust helper binary missing at ${RUST_HELPER_PATH}; run bun run build first`);
    }
    const env = { ...process.env, AXHUB_TOKEN: E2E_TOKEN ?? "", AXHUB_ENDPOINT: E2E_ENDPOINT ?? "" };
    const result = Bun.spawnSync({
      cmd: [RUST_HELPER_PATH, "list-deployments", "--app-id", E2E_APP_ID, "--limit", "1"],
      stdout: "pipe",
      stderr: "pipe",
      env,
    });
    const stdout = result.stdout.toString();
    expect(result.exitCode, result.stderr.toString()).toBe(0);
    const parsed = JSON.parse(stdout) as {
      deployments?: unknown[];
      endpoint_used?: string;
      exit_code?: number;
      error_code?: string;
    };
    expect(parsed.exit_code).toBe(0);
    expect(parsed.error_code).toBeUndefined();
    expect(parsed.endpoint_used).toBe(E2E_ENDPOINT);
    expect(Array.isArray(parsed.deployments)).toBe(true);
  });
});

// When E2E disabled, run a single placeholder test so the test runner shows
// the file as "skipped" rather than empty (clearer signal).
describe.skipIf(E2E_ENABLED)("ax-hub-cli staging E2E (skipped — no AXHUB_E2E_STAGING_TOKEN)", () => {
  test("placeholder: set AXHUB_E2E_STAGING_TOKEN + AXHUB_E2E_STAGING_ENDPOINT to enable", () => {
    expect(E2E_ENABLED).toBe(false);
  });
});
