/**
 * Phase 0 deploy walltime baseline.
 *
 * Spec: .plan/deploy-time-reduction/phase-0-measurement-first.md
 *
 * Phase 0 v1 measures `axhub-helpers session-start` walltime as a proxy for
 * SessionStart cost (Bottleneck B-06). Full deploy walltime (resolve →
 * bootstrap → deploy_create → watch) requires a deploy entry-point that
 * doesn't exist on the helper binary today; subsequent phases can extend
 * each scenario to drive the full cascade once that surface lands.
 *
 * Three scenarios:
 *   1. warm — primed cache, auth signal OK
 *   2. cold — empty HOME (no caches)
 *   3. unauth — empty HOME + token absent (forces UNAUTHORIZED path)
 *
 * Each scenario runs 5 iterations and reports avg + p95. Results land in
 * test-results.json so scripts/perf-parse-results.ts and
 * scripts/perf-publish-measurements.ts can consume them in CI.
 */

import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawn } from "node:child_process";
import { MockBackendServer } from "./fixtures/mock-backend";

const HELPER_BIN_DEFAULT = process.platform === "win32"
  ? join(process.cwd(), "bin", "axhub-helpers.exe")
  : join(process.cwd(), "bin", "axhub-helpers");
const HELPER_BIN = process.env.AXHUB_HELPER_BIN ?? HELPER_BIN_DEFAULT;
const RUNS_PER_SCENARIO = 5;
const RESULTS_FILE = process.env.PERF_RESULTS_FILE
  ?? join(process.cwd(), "test-results.json");

interface RunRecord {
  walltime_ms: number;
  exit_code: number;
}

interface ScenarioRecord {
  avg: number;
  p95: number;
  runs: RunRecord[];
  scope_note: string;
}

type ResultsFile = Record<string, ScenarioRecord>;

function avg(values: number[]): number {
  if (values.length === 0) return 0;
  return Math.round(values.reduce((a, b) => a + b, 0) / values.length);
}

function percentile(values: number[], p: number): number {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const idx = Math.min(sorted.length - 1, Math.ceil((p / 100) * sorted.length) - 1);
  return sorted[Math.max(0, idx)];
}

async function runHelperOnce(
  args: string[],
  env: NodeJS.ProcessEnv,
): Promise<RunRecord> {
  return await new Promise((resolve) => {
    const start = process.hrtime.bigint();
    const child = spawn(HELPER_BIN, args, {
      env,
      stdio: ["ignore", "ignore", "ignore"],
    });
    child.on("exit", (code) => {
      const end = process.hrtime.bigint();
      const walltime_ms = Number((end - start) / 1_000_000n);
      resolve({ walltime_ms, exit_code: code ?? -1 });
    });
    child.on("error", () => {
      const end = process.hrtime.bigint();
      const walltime_ms = Number((end - start) / 1_000_000n);
      resolve({ walltime_ms, exit_code: 127 });
    });
  });
}

async function appendMeasurement(
  scenarioId: string,
  record: ScenarioRecord,
): Promise<void> {
  let prev: ResultsFile = {};
  try {
    const raw = await readFile(RESULTS_FILE, "utf8");
    prev = JSON.parse(raw) as ResultsFile;
  } catch {
    prev = {};
  }
  const next = { ...prev, [scenarioId]: record };
  await writeFile(RESULTS_FILE, JSON.stringify(next, null, 2) + "\n", "utf8");
}

async function makeWorkdir(prefix: string): Promise<string> {
  const dir = await mkdtemp(join(tmpdir(), `axhub-perf-${prefix}-`));
  return dir;
}

async function teardownWorkdir(dir: string): Promise<void> {
  await rm(dir, { recursive: true, force: true });
}

let mock: MockBackendServer;
let mockBaseUrl: string;

beforeAll(async () => {
  mock = new MockBackendServer(0);
  const started = await mock.start();
  mockBaseUrl = started.baseUrl;
});

afterAll(async () => {
  await mock.stop();
});

async function runScenario(
  scenarioId: string,
  setup: () => Promise<{ env: NodeJS.ProcessEnv; cleanup: () => Promise<void> }>,
  hardLimitMs: number,
): Promise<ScenarioRecord> {
  const runs: RunRecord[] = [];
  for (let i = 0; i < RUNS_PER_SCENARIO; i++) {
    const { env, cleanup } = await setup();
    try {
      const r = await runHelperOnce(["session-start"], env);
      runs.push(r);
    } finally {
      await cleanup();
    }
  }
  const wall = runs.map((r) => r.walltime_ms);
  const record: ScenarioRecord = {
    avg: avg(wall),
    p95: percentile(wall, 95),
    runs,
    scope_note:
      "Phase 0 v1: SessionStart walltime only. Full deploy cascade requires future deploy entry-point.",
  };
  await appendMeasurement(scenarioId, record);
  expect(record.avg).toBeLessThan(hardLimitMs);
  return record;
}

describe("deploy walltime baseline", () => {
  test("scenario 1: warm redeploy (primed cache, auth OK)", async () => {
    const record = await runScenario(
      "scenario-1-warm-redeploy",
      async () => {
        const home = await makeWorkdir("warm");
        await mkdir(join(home, ".config", "axhub"), { recursive: true });
        await writeFile(
          join(home, ".config", "axhub", "token.json"),
          JSON.stringify({ token: "valid", expires_at: "2099-01-01T00:00:00Z" }),
        );
        const env: NodeJS.ProcessEnv = {
          ...process.env,
          HOME: home,
          AXHUB_PERF_AUTO_APPROVE: "1",
          AXHUB_TELEMETRY: "1",
          MOCK_BACKEND_LATENCY_MS: "5000",
          AXHUB_BACKEND_URL: mockBaseUrl,
        };
        return { env, cleanup: () => teardownWorkdir(home) };
      },
      120_000,
    );
    expect(record.runs).toHaveLength(RUNS_PER_SCENARIO);
  }, 120_000);

  test("scenario 2: cold first deploy (empty HOME, no caches)", async () => {
    const record = await runScenario(
      "scenario-2-cold-first-deploy",
      async () => {
        const home = await makeWorkdir("cold");
        const env: NodeJS.ProcessEnv = {
          ...process.env,
          HOME: home,
          AXHUB_PERF_AUTO_APPROVE: "1",
          AXHUB_TELEMETRY: "1",
          MOCK_BACKEND_LATENCY_MS: "5000",
          AXHUB_BACKEND_URL: mockBaseUrl,
        };
        return { env, cleanup: () => teardownWorkdir(home) };
      },
      300_000,
    );
    expect(record.runs).toHaveLength(RUNS_PER_SCENARIO);
  }, 300_000);

  test("scenario 3: first deploy + UNAUTHORIZED (no token)", async () => {
    const record = await runScenario(
      "scenario-3-fresh-home-no-token",
      async () => {
        const home = await makeWorkdir("unauth");
        const env: NodeJS.ProcessEnv = {
          ...process.env,
          HOME: home,
          AXHUB_PERF_AUTO_APPROVE: "1",
          AXHUB_TELEMETRY: "1",
          MOCK_BACKEND_LATENCY_MS: "5000",
          AXHUB_BACKEND_URL: mockBaseUrl,
        };
        return { env, cleanup: () => teardownWorkdir(home) };
      },
      600_000,
    );
    expect(record.runs).toHaveLength(RUNS_PER_SCENARIO);
  }, 600_000);
});
