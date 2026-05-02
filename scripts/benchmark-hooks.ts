#!/usr/bin/env bun
/**
 * M4 hook latency benchmark.
 *
 * Measures the hot no-op paths that run on every non-axhub Bash command:
 * - PreToolUse: `preauth-check` should allow a non-destructive Bash command.
 * - PostToolUse: `classify-exit` should emit `{}` for a non-axhub Bash command.
 *
 * The original PLAN text demanded 5ms p95, but audit row 16 accepted the
 * realistic compiled-helper gate: sub-50ms p95. This script makes that gate
 * reproducible without adding a flaky unit-test timing assertion.
 */
import { execFileSync, spawnSync } from "node:child_process";
import { chmodSync, existsSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const helperCandidates = [join(REPO_ROOT, "bin", "axhub-helpers"), join(REPO_ROOT, "bin", "axhub-helpers.exe")];
const findHelper = (): string | null => helperCandidates.find((candidate) => existsSync(candidate)) ?? null;
let helper = findHelper();

const samples = Number(process.env["AXHUB_HOOK_LATENCY_SAMPLES"] ?? "40");
const warmup = Number(process.env["AXHUB_HOOK_LATENCY_WARMUP"] ?? "5");
const thresholdMs = Number(process.env["AXHUB_HOOK_LATENCY_P95_MS"] ?? "50");
const shouldBuild = !process.argv.includes("--no-build");
const printConfigOnly = process.argv.includes("--print-config");
const fakeBinDir = mkdtempSync(join(tmpdir(), "axhub-hook-bench-"));
const fakeAxhub = join(fakeBinDir, "axhub");
writeFileSync(
  fakeAxhub,
  `#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "axhub 0.10.2 (bench)"
  exit 0
fi
if [ "$1" = "auth" ] && [ "$2" = "status" ] && [ "$3" = "--json" ]; then
  echo '{"user_email":"bench@example.com","user_id":1,"expires_at":"2099-01-01T00:00:00Z","scopes":["read","deploy"]}'
  exit 0
fi
exit 0
`,
);
chmodSync(fakeAxhub, 0o755);

const scenarios = [
  {
    name: "preauth-check non-axhub Bash allow",
    subcommand: "preauth-check",
    thresholdMs: 30,
    payload: {
      hook_event_name: "PreToolUse",
      tool_name: "Bash",
      tool_input: { command: "echo hello" },
    },
    assertOutput(stdout: string): void {
      const parsed = JSON.parse(stdout);
      const decision = parsed?.hookSpecificOutput?.permissionDecision;
      if (decision !== "allow") throw new Error(`expected PreToolUse allow, got ${stdout}`);
    },
  },
  {
    name: "classify-exit non-axhub Bash no-op",
    subcommand: "classify-exit",
    thresholdMs: 10,
    payload: {
      hook_event_name: "PostToolUse",
      tool_name: "Bash",
      tool_input: { command: "echo hello" },
      tool_response: { exit_code: 0, stdout: "hello\n", stderr: "" },
    },
    assertOutput(stdout: string): void {
      const parsed = JSON.parse(stdout);
      if (Object.keys(parsed).length !== 0) throw new Error(`expected PostToolUse no-op {}, got ${stdout}`);
    },
  },
  {
    name: "prompt-route open no-preflight",
    subcommand: "prompt-route",
    thresholdMs: 50,
    payload: { hook_event_name: "UserPromptSubmit", prompt: "결과 봐" },
    assertOutput(stdout: string): void {
      if (!stdout.includes("skills/open/SKILL.md")) throw new Error(`expected open route, got ${stdout}`);
    },
  },
  {
    name: "prompt-route whatsnew no-preflight",
    subcommand: "prompt-route",
    thresholdMs: 50,
    payload: { hook_event_name: "UserPromptSubmit", prompt: "axhub 뭐 새로 나왔어" },
    assertOutput(stdout: string): void {
      if (!stdout.includes("skills/whatsnew/SKILL.md")) throw new Error(`expected whatsnew route, got ${stdout}`);
    },
  },
  {
    name: "prompt-route profile current no-preflight",
    subcommand: "prompt-route",
    thresholdMs: 50,
    payload: { hook_event_name: "UserPromptSubmit", prompt: "profile current" },
    assertOutput(stdout: string): void {
      if (!stdout.includes("skills/profile/SKILL.md")) throw new Error(`expected profile route, got ${stdout}`);
    },
  },
  {
    name: "prompt-route env list with preflight",
    subcommand: "prompt-route",
    thresholdMs: 50,
    env: { AXHUB_BIN: fakeAxhub },
    payload: { hook_event_name: "UserPromptSubmit", prompt: "env list" },
    assertOutput(stdout: string): void {
      if (!stdout.includes("skills/env/SKILL.md") || !stdout.includes("Preflight 결과")) {
        throw new Error(`expected env route with preflight, got ${stdout}`);
      }
    },
  },
  {
    name: "prompt-route github connect with preflight",
    subcommand: "prompt-route",
    thresholdMs: 50,
    env: { AXHUB_BIN: fakeAxhub },
    payload: { hook_event_name: "UserPromptSubmit", prompt: "github connect" },
    assertOutput(stdout: string): void {
      if (!stdout.includes("skills/github/SKILL.md") || !stdout.includes("Preflight 결과")) {
        throw new Error(`expected github route with preflight, got ${stdout}`);
      }
    },
  },
  {
    name: "prompt-route deploy with preflight",
    subcommand: "prompt-route",
    thresholdMs: 50,
    env: { AXHUB_BIN: fakeAxhub },
    payload: { hook_event_name: "UserPromptSubmit", prompt: "deploy" },
    assertOutput(stdout: string): void {
      if (!stdout.includes("skills/deploy/SKILL.md") || !stdout.includes("Preflight 결과")) {
        throw new Error(`expected deploy route with preflight, got ${stdout}`);
      }
    },
  },
  {
    name: "prompt-route clarify fallback",
    subcommand: "prompt-route",
    thresholdMs: 50,
    payload: { hook_event_name: "UserPromptSubmit", prompt: "환경" },
    assertOutput(stdout: string): void {
      if (!stdout.includes("skills/clarify/SKILL.md")) throw new Error(`expected clarify route, got ${stdout}`);
    },
  },
] as const;

type Result = { name: string; p50: number; p95: number; max: number };

const fail = (message: string): never => {
  process.stderr.write(`[hook-latency] FAIL: ${message}\n`);
  process.exit(1);
};

const percentile = (values: number[], p: number): number => {
  const index = Math.min(values.length - 1, Math.ceil(values.length * p) - 1);
  return values[index] ?? 0;
};

const runScenario = (scenario: typeof scenarios[number]): Result => {
  const input = JSON.stringify(scenario.payload);
  const times: number[] = [];

  for (let i = 0; i < samples + warmup; i += 1) {
    const started = process.hrtime.bigint();
    if (!helper) fail("helper binary missing");
    const result = spawnSync(helper, [scenario.subcommand], {
      cwd: REPO_ROOT,
      input,
      encoding: "utf8",
      env: { ...process.env, AXHUB_TELEMETRY: "0", ...(scenario as { env?: Record<string, string> }).env },
    });
    const elapsedMs = Number(process.hrtime.bigint() - started) / 1_000_000;

    if (result.status !== 0) {
      fail(`${scenario.name}: exit ${result.status}; stderr=${result.stderr.trim()}`);
    }
    try {
      scenario.assertOutput(result.stdout.trim());
    } catch (err) {
      fail(`${scenario.name}: ${err instanceof Error ? err.message : String(err)}`);
    }
    if (i >= warmup) times.push(elapsedMs);
  }

  times.sort((a, b) => a - b);
  return {
    name: scenario.name,
    p50: percentile(times, 0.50),
    p95: percentile(times, 0.95),
    max: times[times.length - 1] ?? 0,
  };
};

if (printConfigOnly) {
  process.stdout.write(JSON.stringify({ samples, warmup, thresholdMs, scenarios: scenarios.map((s) => ({ name: s.name, subcommand: s.subcommand, thresholdMs: s.thresholdMs ?? thresholdMs })) }, null, 2) + "\n");
  process.exit(0);
}

if (samples < 5) fail("AXHUB_HOOK_LATENCY_SAMPLES must be >= 5");
if (warmup < 0) fail("AXHUB_HOOK_LATENCY_WARMUP must be >= 0");
if (!Number.isFinite(thresholdMs) || thresholdMs <= 0) fail("AXHUB_HOOK_LATENCY_P95_MS must be > 0");

if (shouldBuild || !helper) {
  execFileSync("bun", ["run", "build"], { cwd: REPO_ROOT, stdio: "inherit" });
  helper = findHelper();
}
if (!helper) fail(`helper binary missing after build: ${helperCandidates.join(", ")}`);

process.stdout.write(`[hook-latency] samples=${samples} warmup=${warmup} p95-threshold=${thresholdMs}ms\n`);
const results = scenarios.map(runScenario);
for (const r of results) {
  const scenario = scenarios.find((s) => s.name === r.name);
  const scenarioThreshold = scenario?.thresholdMs ?? thresholdMs;
  process.stdout.write(`[hook-latency] ${r.name}: p50=${r.p50.toFixed(2)}ms p95=${r.p95.toFixed(2)}ms max=${r.max.toFixed(2)}ms threshold=${scenarioThreshold}ms\n`);
  if (r.p95 > scenarioThreshold) {
    fail(`${r.name}: p95 ${r.p95.toFixed(2)}ms > ${scenarioThreshold}ms`);
  }
}
process.stdout.write("[hook-latency] OK\n");
