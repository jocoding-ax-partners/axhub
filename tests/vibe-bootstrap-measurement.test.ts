import { describe, expect, test } from "bun:test";
import type { SpawnSyncReturns } from "node:child_process";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  artifactContainsForbiddenValue,
  buildMeasurementSummary,
  DEFAULT_FIXTURE_APP,
  parseTranscriptMetrics,
  percentile,
  runLiveMeasurement,
  sanitizeArtifact,
  validateMeasurementEnv,
  verifyOpenUrlPresence,
  verifyReadinessStatus,
  type MeasurementConfig,
  type MeasurementRun,
} from "../scripts/measure-vibe-bootstrap.ts";
import { evaluateSla } from "../scripts/check-vibe-bootstrap-sla.ts";

const fakeHelper = (): string => {
  const dir = mkdtempSync(join(tmpdir(), "axhub-helper-test-"));
  const path = join(dir, "axhub-helpers");
  writeFileSync(path, "#!/bin/sh\nexit 0\n");
  return path;
};

type FakeRunner = NonNullable<Parameters<typeof runLiveMeasurement>[1]>;

const spawnResult = (stdout = "{}", status = 0, stderr = ""): SpawnSyncReturns<string> =>
  ({
    pid: 0,
    output: [null, stdout, stderr],
    stdout,
    stderr,
    status,
    signal: null,
  });

const measurementConfig = (overrides: Partial<MeasurementConfig> = {}): MeasurementConfig => ({
  token: "axhub_pat_test",
  endpoint: "https://staging.example.test",
  destructive: true,
  maxRuns: 1,
  costBudgetUsd: 1,
  cleanupMode: "ttl",
  fixtureApp: DEFAULT_FIXTURE_APP,
  helperPath: "/tmp/fake-axhub-helpers",
  watchTimeout: "1m",
  ...overrides,
});

describe("vibe bootstrap measurement env gate", () => {
  test("requires explicit token, endpoint, destructive opt-in, budget, and cleanup mode", () => {
    expect(() => validateMeasurementEnv({})).toThrow("AXHUB_E2E_STAGING_TOKEN");
    expect(() =>
      validateMeasurementEnv({
        AXHUB_E2E_STAGING_TOKEN: "axhub_pat_test",
        AXHUB_E2E_STAGING_ENDPOINT: "https://staging.example.test",
        AXHUB_E2E_DESTRUCTIVE: "0",
      }),
    ).toThrow("AXHUB_E2E_DESTRUCTIVE=1");
  });

  test("accepts only fully bounded destructive measurement config", () => {
    const helper = fakeHelper();
    const config = validateMeasurementEnv({
      AXHUB_E2E_STAGING_TOKEN: "axhub_pat_test",
      AXHUB_E2E_STAGING_ENDPOINT: "https://staging.example.test",
      AXHUB_E2E_DESTRUCTIVE: "1",
      AXHUB_E2E_MAX_RUNS: "2",
      AXHUB_E2E_COST_BUDGET_USD: "1.5",
      AXHUB_E2E_CLEANUP_MODE: "ttl",
      AXHUB_E2E_FIXTURE_APP: DEFAULT_FIXTURE_APP,
      AXHUB_E2E_HELPER_PATH: helper,
    });
    expect(config.maxRuns).toBe(2);
    expect(config.cleanupMode).toBe("ttl");
    expect(config.endpoint).toBe("https://staging.example.test");
  });
});

describe("vibe bootstrap measurement pure helpers", () => {
  test("percentile uses nearest-rank semantics", () => {
    expect(percentile([], 0.95)).toBe(0);
    expect(percentile([3], 0.95)).toBe(3);
    expect(percentile([30, 10, 20, 40], 0.5)).toBe(20);
    expect(percentile([30, 10, 20, 40], 0.95)).toBe(40);
  });

  test("parses ask and consent metrics from transcript events", () => {
    const metrics = parseTranscriptMetrics([
      JSON.stringify({ event: "AskUserQuestion" }),
      JSON.stringify({ event: "ask_user_question" }),
      JSON.stringify({ event: "consent_mint" }),
      JSON.stringify({ event: "preauth_check_deny" }),
      JSON.stringify({ hookSpecificOutput: { permissionDecision: "deny" } }),
      "not-json",
    ]);
    expect(metrics).toEqual({ ask_count: 2, consent_mint_count: 1, consent_block_count: 2 });
  });

  test("verifies readiness and live URL presence without returning raw URL", () => {
    expect(verifyReadinessStatus({ deployment: { status: "succeeded" } }).ok).toBe(true);
    expect(verifyReadinessStatus({ status: "failed" }).ok).toBe(false);
    const open = verifyOpenUrlPresence({ deploy_url: "https://app.example.test/path" });
    expect(open.live_url_present).toBe(true);
    expect(open.live_url_host_hash).toHaveLength(16);
    expect(JSON.stringify(open)).not.toContain("app.example.test");
  });

  test("redacts forbidden artifact fields and token/url/email strings", () => {
    const artifact = sanitizeArtifact({
      token: "axhub_pat_secret",
      user_email: "dev@example.com",
      nested: { safe: "ok", note: "visit https://app.example.test" },
    });
    expect(JSON.stringify(artifact)).not.toContain("axhub_pat_secret");
    expect(JSON.stringify(artifact)).not.toContain("dev@example.com");
    expect(JSON.stringify(artifact)).not.toContain("https://app.example.test");
    expect(artifactContainsForbiddenValue(artifact)).toBe(false);
  });
});

describe("vibe bootstrap live measurement runner", () => {
  test("drives a fake helper/CLI success path without leaking command artifacts", () => {
    let planStep = 0;
    const recordedEnvelopes: Array<Record<string, unknown>> = [];
    const commands: string[] = [];
    const runner: FakeRunner = (cmd, opts) => {
      commands.push(cmd.join(" "));
      if (cmd[0] === "/tmp/fake-axhub-helpers" && cmd[1] === "bootstrap" && cmd[2] === "--record") {
        recordedEnvelopes.push(JSON.parse(opts.input ?? "{}") as Record<string, unknown>);
        return spawnResult(JSON.stringify({ state: cmd[3] === "apps_create" ? "needs_deploy_create" : "deploying" }));
      }
      if (cmd[0] === "/tmp/fake-axhub-helpers" && cmd[1] === "bootstrap") {
        planStep += 1;
        if (planStep === 1) {
          return spawnResult(JSON.stringify({ state: "needs_git_init", next_action: "git_init", command: ["git", "init"] }));
        }
        if (planStep === 2) {
          return spawnResult(JSON.stringify({ state: "needs_first_commit", next_action: "first_commit", command: ["git", "commit", "-m", "init"] }));
        }
        if (planStep === 3) {
          return spawnResult(JSON.stringify({
            state: "needs_apps_create",
            next_action: "apps_create",
            pending_action_id: "pending-app",
            pending_action_hash: "hash-app",
            command: ["axhub", "apps", "create", "--json"],
          }));
        }
        if (planStep === 4) {
          return spawnResult(JSON.stringify({
            state: "needs_deploy_create",
            next_action: "deploy_create",
            pending_action_id: "pending-deploy",
            pending_action_hash: "hash-deploy",
            command: ["axhub", "deploy", "create", "--json"],
          }));
        }
        mkdirSync(join(opts.cwd, ".axhub"), { recursive: true });
        writeFileSync(join(opts.cwd, ".axhub/bootstrap.state.json"), JSON.stringify({ state: "deploying", app_id: "app-1", last_deploy_id: "dep-1" }));
        return spawnResult(JSON.stringify({ state: "deploying" }));
      }
      if (cmd[0] === "git") return spawnResult("{}");
      if (cmd[0] === "axhub" && cmd[1] === "apps") return spawnResult(JSON.stringify({ id: "app-1" }));
      if (cmd[0] === "axhub" && cmd[1] === "deploy" && cmd[2] === "create") {
        return spawnResult(JSON.stringify({ id: "dep-1" }));
      }
      if (cmd[0] === "axhub" && cmd[1] === "deploy" && cmd[2] === "status") {
        return spawnResult(JSON.stringify({ status: "succeeded" }));
      }
      if (cmd[0] === "axhub" && cmd[1] === "open") {
        return spawnResult(JSON.stringify({ deploy_url: "https://live.example.test" }));
      }
      return spawnResult("{}", 1, `unexpected command: ${cmd.join(" ")}`);
    };

    const summary = runLiveMeasurement(measurementConfig(), runner);

    expect(summary.failure_count).toBe(0);
    expect(summary.sample_size).toBe(1);
    expect(summary.live_url_present).toBe(true);
    expect(summary.consent_mint_count).toBe(2);
    expect(recordedEnvelopes).toHaveLength(2);
    expect(recordedEnvelopes[0]?.["command_argv"]).toEqual(["axhub", "apps", "create", "--json"]);
    expect(commands).toContain("axhub deploy status dep-1 --app app-1 --watch --watch-timeout 1m --json");
    expect(artifactContainsForbiddenValue(summary)).toBe(false);
  });

  test("returns typed blocker and unsupported-command failures without raw command leakage", () => {
    const blockerSummary = runLiveMeasurement(measurementConfig(), (cmd) => {
      if (cmd[0] === "/tmp/fake-axhub-helpers") {
        return spawnResult(JSON.stringify({ state: "needs_apps_create" }));
      }
      return spawnResult("{}");
    });
    expect(blockerSummary.failure_count).toBe(1);
    expect(blockerSummary.runs[0]?.failure_code).toBe("bootstrap.typed_blocker");
    expect(artifactContainsForbiddenValue(blockerSummary)).toBe(false);

    const unsupportedSummary = runLiveMeasurement(measurementConfig(), (cmd) => {
      if (cmd[0] === "/tmp/fake-axhub-helpers") {
        return spawnResult(JSON.stringify({
          state: "needs_custom_command",
          next_action: "custom",
          command: ["curl", "https://forbidden.example.test"],
        }));
      }
      return spawnResult("{}");
    });
    expect(unsupportedSummary.failure_count).toBe(1);
    expect(unsupportedSummary.runs[0]?.failure_code).toBe("command.unsupported");
    expect(artifactContainsForbiddenValue(unsupportedSummary)).toBe(false);
  });

  test("returns readiness failures without opening a live URL", () => {
    const summary = runLiveMeasurement(measurementConfig(), (cmd, opts) => {
      if (cmd[0] === "/tmp/fake-axhub-helpers") {
        mkdirSync(join(opts.cwd, ".axhub"), { recursive: true });
        writeFileSync(join(opts.cwd, ".axhub/bootstrap.state.json"), JSON.stringify({ state: "deploying", app_id: "app-1", last_deploy_id: "dep-1" }));
        return spawnResult(JSON.stringify({ state: "deploying" }));
      }
      if (cmd[0] === "axhub" && cmd[1] === "deploy" && cmd[2] === "status") {
        return spawnResult(JSON.stringify({ status: "failed" }));
      }
      if (cmd[0] === "axhub" && cmd[1] === "open") {
        throw new Error("open must not run when readiness failed");
      }
      return spawnResult("{}");
    });

    expect(summary.failure_count).toBe(1);
    expect(summary.runs[0]?.failure_code).toBe("readiness.not_ready");
    expect(summary.live_url_present).toBe(false);
    expect(artifactContainsForbiddenValue(summary)).toBe(false);
  });
});

describe("vibe bootstrap summary and SLA gate", () => {
  const run = (seconds: number): MeasurementRun => ({
    run_id: `run-${seconds}`,
    success: true,
    phase_durations_ms: { bootstrap_plan: 10, deploy_watch: seconds * 1000 },
    total_seconds: seconds,
    ask_count: 1,
    consent_mint_count: 2,
    consent_block_count: 0,
    readiness_source: "deploy_status",
    live_url_present: true,
    live_url_host_hash: "0123456789abcdef",
    cleanup_mode: "ttl",
  });

  test("builds redacted summary with p50 and p95", () => {
    const summary = buildMeasurementSummary(
      [run(100), run(200), run(300)],
      "https://staging.example.test",
      "ttl",
      "2026-05-06T00:00:00.000Z",
      "2026-05-06T00:05:00.000Z",
    );
    expect(summary.sample_size).toBe(3);
    expect(summary.p50_seconds).toBe(200);
    expect(summary.p95_seconds).toBe(300);
    expect(summary.ask_count).toBe(3);
    expect(summary.consent_block_count).toBe(0);
    expect(artifactContainsForbiddenValue(summary)).toBe(false);
  });

  test("advisory SLA warns while blocking SLA fails", () => {
    const summary = buildMeasurementSummary(
      [run(500)],
      "https://staging.example.test",
      "ttl",
      "2026-05-06T00:00:00.000Z",
      "2026-05-06T00:10:00.000Z",
    );
    const advisory = evaluateSla(summary, { p95Seconds: 480, minSamples: 20, mode: "advisory" });
    expect(advisory.pass).toBe(true);
    expect(advisory.warnings.length).toBeGreaterThan(0);
    const blocking = evaluateSla(summary, { p95Seconds: 480, minSamples: 20, mode: "blocking" });
    expect(blocking.pass).toBe(false);
    expect(blocking.errors.length).toBeGreaterThan(0);
  });
});
