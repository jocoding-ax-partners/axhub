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

const E2E_TOKEN = process.env["AXHUB_E2E_STAGING_TOKEN"];
const E2E_ENDPOINT = process.env["AXHUB_E2E_STAGING_ENDPOINT"];

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

const extractAppsList = (parsed: unknown): unknown[] => {
  if (Array.isArray(parsed)) return parsed;
  if (parsed && typeof parsed === "object") {
    const obj = parsed as Record<string, unknown>;
    // ax-hub-cli has returned both {apps:[...]} and {data:[...]} across releases.
    // Keep staging E2E focused on the read contract instead of one envelope name.
    for (const key of ["apps", "data"]) {
      const value = obj[key];
      if (Array.isArray(value)) return value;
    }
  }
  const preview = JSON.stringify(parsed) ?? String(parsed);
  throw new Error(`unexpected apps list shape: ${preview.slice(0, 200)}`);
};

describe("staging E2E response shape helpers", () => {
  test("extractAppsList accepts all supported CLI envelopes", () => {
    const app = { id: 1, slug: "demo" };
    expect(extractAppsList([app])).toEqual([app]);
    expect(extractAppsList({ apps: [app] })).toEqual([app]);
    expect(extractAppsList({ data: [app] })).toEqual([app]);
  });

  test("extractAppsList rejects unknown envelopes", () => {
    expect(() => extractAppsList({ items: [] })).toThrow("unexpected apps list shape");
  });
});

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
    expectAppListContract(extractAppsList(parsed));
  });

  // The Rust helper (bin/axhub-helpers, crates/axhub-helpers) staging probe was
  // removed with the helper binary during the US-010 diet — its cargo-side
  // coverage went away with the crate, so there is nothing left to shadow here.
});

// When E2E disabled, run a single placeholder test so the test runner shows
// the file as "skipped" rather than empty (clearer signal).
describe.skipIf(E2E_ENABLED)("ax-hub-cli staging E2E (skipped — no AXHUB_E2E_STAGING_TOKEN)", () => {
  test("placeholder: set AXHUB_E2E_STAGING_TOKEN + AXHUB_E2E_STAGING_ENDPOINT to enable", () => {
    expect(E2E_ENABLED).toBe(false);
  });
});
