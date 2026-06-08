/**
 * Phase 0.5 deploy walltime baseline.
 *
 * Spec: .plan/deploy-time-reduction/phase-0-measurement-first.md (§10.1).
 *
 * Drives the deploy SKILL Step 0..5 cascade via tests/perf/scripts/
 * deploy-flow-harness.sh. The harness records markers through
 * `axhub-helpers mark` and emits the final telemetry envelope through
 * `axhub-helpers emit-deploy-complete`. Mock backend latency is the
 * dominant walltime contributor (4 routes × MOCK_BACKEND_LATENCY_MS).
 *
 * Three scenarios:
 *   1. warm — primed cache, auth signal OK
 *   2. cold — empty HOME (no caches)
 *   3. fresh-HOME — empty HOME + token absent
 *
 * Each scenario runs 5 iterations and reports avg + p95. Results land in
 * test-results.json so scripts/perf-parse-results.ts and
 * scripts/perf-publish-measurements.ts can consume them in CI.
 */

import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdir, mkdtemp, readFile, rm, unlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawn } from "node:child_process";
import { MockBackendServer } from "./fixtures/mock-backend";

const HELPER_BIN_DEFAULT =
  process.platform === "win32"
    ? join(process.cwd(), "bin", "axhub-helpers.exe")
    : join(process.cwd(), "bin", "axhub-helpers");
const HELPER_BIN = process.env.AXHUB_HELPER_BIN ?? HELPER_BIN_DEFAULT;
const HARNESS_PATH = join(
  process.cwd(),
  "tests",
  "perf",
  "scripts",
  "deploy-flow-harness.sh",
);
const RUNS_PER_SCENARIO = 5;
const RESULTS_FILE =
  process.env.PERF_RESULTS_FILE ?? join(process.cwd(), "test-results.json");
const LOCAL_DEFAULT_LATENCY_MS = "200";

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

async function runHarnessOnce(env: NodeJS.ProcessEnv): Promise<RunRecord> {
  return await new Promise((resolve) => {
    const start = process.hrtime.bigint();
    const child = spawn("bash", [HARNESS_PATH], {
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

async function safeUnlink(path: string): Promise<void> {
  try {
    await unlink(path);
  } catch {
    // ignore — file may already be drained by emit-deploy-complete
  }
}

let mock: MockBackendServer;
let mockBaseUrl: string;

beforeAll(async () => {
  // CI passes MOCK_BACKEND_LATENCY_MS=5000; local default keeps test fast.
  if (!process.env.MOCK_BACKEND_LATENCY_MS) {
    process.env.MOCK_BACKEND_LATENCY_MS = LOCAL_DEFAULT_LATENCY_MS;
  }
  mock = new MockBackendServer();
  const started = await mock.start();
  mockBaseUrl = started.baseUrl;
});

afterAll(async () => {
  await mock.stop();
});

interface ScenarioSetup {
  env: NodeJS.ProcessEnv;
  cleanup: () => Promise<void>;
}

async function buildEnv(
  prefix: string,
  extras: NodeJS.ProcessEnv,
): Promise<ScenarioSetup> {
  const home = await makeWorkdir(prefix);
  const markerFile = join(home, "phase-markers.jsonl");
  const env: NodeJS.ProcessEnv = {
    ...process.env,
    HOME: home,
    AXHUB_HELPER_BIN: HELPER_BIN,
    AXHUB_PHASE_MARKER_FILE: markerFile,
    AXHUB_PERF_AUTO_APPROVE: "1",
    AXHUB_TELEMETRY: "1",
    AXHUB_BACKEND_URL: mockBaseUrl,
    XDG_STATE_HOME: join(home, ".local", "state"),
    ...extras,
  };
  return {
    env,
    cleanup: async () => {
      await safeUnlink(markerFile);
      await teardownWorkdir(home);
    },
  };
}

async function runScenario(
  scenarioId: string,
  setup: () => Promise<ScenarioSetup>,
  hardLimitMs: number,
): Promise<ScenarioRecord> {
  const runs: RunRecord[] = [];
  for (let i = 0; i < RUNS_PER_SCENARIO; i++) {
    const { env, cleanup } = await setup();
    try {
      const r = await runHarnessOnce(env);
      expect(r.exit_code).toBe(0);
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
      "Phase 0.5: full deploy cascade walltime via deploy-flow-harness.sh (mock backend).",
  };
  await appendMeasurement(scenarioId, record);
  expect(record.avg).toBeLessThan(hardLimitMs);
  return record;
}

describe("deploy walltime baseline (Phase 0.5)", () => {
  test("scenario 1: warm redeploy (primed cache, auth OK)", async () => {
    const record = await runScenario(
      "scenario-1-warm-redeploy",
      async () => {
        const setup = await buildEnv("warm", {});
        await mkdir(join(setup.env.HOME!, ".config", "axhub"), {
          recursive: true,
        });
        await writeFile(
          join(setup.env.HOME!, ".config", "axhub", "token.json"),
          JSON.stringify({ token: "valid", expires_at: "2099-01-01T00:00:00Z" }),
        );
        return setup;
      },
      120_000,
    );
    expect(record.runs).toHaveLength(RUNS_PER_SCENARIO);
  }, 120_000);

  test("scenario 2: cold first deploy (empty HOME, no caches)", async () => {
    const record = await runScenario(
      "scenario-2-cold-first-deploy",
      async () => buildEnv("cold", {}),
      300_000,
    );
    expect(record.runs).toHaveLength(RUNS_PER_SCENARIO);
  }, 300_000);

  test("scenario 3: fresh HOME + auth-refresh round-trip", async () => {
    const record = await runScenario(
      "scenario-3-fresh-home-no-token",
      async () => buildEnv("fresh-home", { AXHUB_PERF_FORCE_UNAUTH: "1" }),
      600_000,
    );
    expect(record.runs).toHaveLength(RUNS_PER_SCENARIO);
  }, 600_000);
});
