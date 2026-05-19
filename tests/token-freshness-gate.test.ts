/**
 * Phase 3.5 — Token freshness gate Step 3.5 polling consumer.
 *
 * Spec: .plan/deploy-time-reduction/phase-3-client-cascade-reduced.md §3.4.
 *
 * sh/ps1-absorption Phase 4 (F1): test now spawns `axhub-helpers token-gate`
 * directly. The legacy `hooks/token-freshness-gate.sh` shim has been removed —
 * SKILL deploy Step 3.5 calls the helper binary directly, and the bash shim
 * had no remaining caller after T8 SKILL migration.
 *
 * Exercises the gate with controlled token file mtimes and a fake "now"
 * timestamp. The gate is the consumer half of Phase 3.5; the producer
 * (auth-refresh-bg detached spawn) is covered separately by
 * tests/session-start-bg-refresh.test.ts.
 */

import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { spawn } from "node:child_process";
import { mkdtemp, rm, writeFile, utimes } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const GATE_BINARY = join(process.cwd(), "bin", "axhub-helpers");

interface RunResult {
  exitCode: number;
  stdout: string;
  stderr: string;
  walltimeMs: number;
}

async function runGate(env: NodeJS.ProcessEnv): Promise<RunResult> {
  return await new Promise((resolve) => {
    const start = process.hrtime.bigint();
    const child = spawn(GATE_BINARY, ["token-gate"], {
      env: { ...process.env, ...env },
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (b) => (stdout += b.toString()));
    child.stderr.on("data", (b) => (stderr += b.toString()));
    child.on("exit", (code) => {
      const end = process.hrtime.bigint();
      resolve({
        exitCode: code ?? -1,
        stdout,
        stderr,
        walltimeMs: Number((end - start) / 1_000_000n),
      });
    });
    child.on("error", () => {
      const end = process.hrtime.bigint();
      resolve({
        exitCode: 127,
        stdout,
        stderr,
        walltimeMs: Number((end - start) / 1_000_000n),
      });
    });
  });
}

async function setupTokenFile(workdir: string, mtimeSecs: number): Promise<string> {
  const tokenPath = join(workdir, "token");
  await writeFile(tokenPath, "stub-token");
  await utimes(tokenPath, mtimeSecs, mtimeSecs);
  return tokenPath;
}

describe("axhub-helpers token-gate (Phase 3.5 Step 3.5 consumer)", () => {
  let workdir: string;

  beforeEach(async () => {
    workdir = await mkdtemp(join(tmpdir(), "axhub-gate-"));
  });

  afterEach(async () => {
    await rm(workdir, { recursive: true, force: true });
  });

  test("AXHUB_AUTH_BG_REFRESH=0 short-circuits to exit 0", async () => {
    const result = await runGate({
      AXHUB_AUTH_BG_REFRESH: "0",
      AXHUB_TOKEN_PATH: join(workdir, "no-such-file"),
    });
    expect(result.exitCode).toBe(0);
    expect(result.walltimeMs).toBeLessThan(500);
  });

  test("Token missing + UNAUTHORIZED inline check exits 65", async () => {
    const result = await runGate({
      AXHUB_AUTH_BG_REFRESH: "1",
      AXHUB_TOKEN_PATH: join(workdir, "missing-token"),
      AXHUB_GATE_AUTH_PROBE: 'echo "{\\"code\\":\\"auth.token_missing\\"}"',
    });
    expect(result.exitCode).toBe(65);
    expect(result.walltimeMs).toBeLessThan(500);
    expect(result.stderr).toContain("token file missing");
    expect(result.stderr).toContain("UNAUTHORIZED");
  });

  test("Token missing + authenticated inline check exits 0", async () => {
    const result = await runGate({
      AXHUB_AUTH_BG_REFRESH: "1",
      AXHUB_TOKEN_PATH: join(workdir, "missing-token"),
      AXHUB_GATE_AUTH_PROBE:
        'echo "{\\"user_email\\":\\"dev@jocodingax.ai\\",\\"user_id\\":1}"',
    });
    expect(result.exitCode).toBe(0);
    expect(result.walltimeMs).toBeLessThan(500);
    expect(result.stderr).toContain("token file missing");
  });

  test("Fresh token (mtime > now-30) exits 0 without polling", async () => {
    const fakeNow = 2_000_000;
    // mtime = now - 5s → fresh relative to SESSION_TS = now - 30
    const tokenPath = await setupTokenFile(workdir, fakeNow - 5);
    const result = await runGate({
      AXHUB_AUTH_BG_REFRESH: "1",
      AXHUB_TOKEN_PATH: tokenPath,
      AXHUB_GATE_FAKE_NOW: String(fakeNow),
    });
    expect(result.exitCode).toBe(0);
    expect(result.walltimeMs).toBeLessThan(500);
    expect(result.stderr).toContain("fresh");
  });

  test("Stale token + UNAUTHORIZED inline check → exit 65", async () => {
    const fakeNow = 2_000_000;
    // mtime far in the past relative to fakeNow → polling exhausts then inline
    const tokenPath = await setupTokenFile(workdir, fakeNow - 3600);
    const result = await runGate({
      AXHUB_AUTH_BG_REFRESH: "1",
      AXHUB_TOKEN_PATH: tokenPath,
      AXHUB_GATE_FAKE_NOW: String(fakeNow),
      AXHUB_GATE_POLL_INTERVAL: "0",
      AXHUB_GATE_POLL_ITERATIONS: "2",
      // Force inline auth probe to return UNAUTHORIZED.
      AXHUB_GATE_AUTH_PROBE: 'echo "{\\"code\\":\\"auth.token_missing\\"}"',
    });
    expect(result.exitCode).toBe(65);
    expect(result.stderr).toContain("UNAUTHORIZED");
  });

  test("Stale token that becomes fresh during polling exits 0 without inline probe", async () => {
    const fakeNow = 2_000_000;
    const tokenPath = await setupTokenFile(workdir, fakeNow - 3600);
    const gate = runGate({
      AXHUB_AUTH_BG_REFRESH: "1",
      AXHUB_TOKEN_PATH: tokenPath,
      AXHUB_GATE_FAKE_NOW: String(fakeNow),
      AXHUB_GATE_POLL_INTERVAL: "1",
      AXHUB_GATE_POLL_ITERATIONS: "2",
      AXHUB_GATE_AUTH_PROBE: 'echo "{\\"code\\":\\"should_not_run\\"}"; exit 1',
    });
    await new Promise((resolve) => setTimeout(resolve, 150));
    await utimes(tokenPath, fakeNow - 5, fakeNow - 5);

    const result = await gate;
    expect(result.exitCode).toBe(0);
    expect(result.stderr).toContain("token refreshed after");
    expect(result.stderr).not.toContain("inline auth status check");
  });

  test("Stale token + authenticated inline check → exit 0", async () => {
    const fakeNow = 2_000_000;
    const tokenPath = await setupTokenFile(workdir, fakeNow - 3600);
    const result = await runGate({
      AXHUB_AUTH_BG_REFRESH: "1",
      AXHUB_TOKEN_PATH: tokenPath,
      AXHUB_GATE_FAKE_NOW: String(fakeNow),
      AXHUB_GATE_POLL_INTERVAL: "0",
      AXHUB_GATE_POLL_ITERATIONS: "2",
      // Inline probe finds a valid user_email — token must have been refreshed
      // out-of-band; gate proceeds.
      AXHUB_GATE_AUTH_PROBE:
        'echo "{\\"user_email\\":\\"dev@jocodingax.ai\\",\\"user_id\\":1}"',
    });
    expect(result.exitCode).toBe(0);
    expect(result.stderr).toContain("poll timeout");
  });
});
