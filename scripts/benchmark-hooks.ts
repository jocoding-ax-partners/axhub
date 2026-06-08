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
const thresholdMs = Number(process.env["AXHUB_HOOK_LATENCY_P95_MS"] ?? "120");
const shouldBuild = !process.argv.includes("--no-build");
const printConfigOnly = process.argv.includes("--print-config");
const fakeBinDir = mkdtempSync(join(tmpdir(), "axhub-hook-bench-"));
const fakeAxhub = join(fakeBinDir, "axhub");
const missingAxhub = join(fakeBinDir, "axhub-missing");
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
  // Approach E (Phase 2): cmd_prompt_route is preflight + audit only. No skill path
  // enforcement. Latency benchmark for hot-path prompt processing.
  {
    name: "prompt-route no-preflight",
    subcommand: "prompt-route",
    thresholdMs: 120,
    // Keep this hot-path measurement deterministic on developer machines that
    // have a real axhub CLI installed; the fake-preflight scenarios below cover
    // the healthy-CLI path explicitly.
    env: { AXHUB_BIN: missingAxhub },
    payload: { hook_event_name: "UserPromptSubmit", prompt: "결과 봐" },
    assertOutput(stdout: string): void {
      if (stdout.includes("skills/")) throw new Error(`Approach E: no skill path expected, got ${stdout}`);
      if (stdout.includes("워크플로우를 우선 적용")) throw new Error(`Approach E: no enforcement language, got ${stdout}`);
    },
  },
  {
    name: "prompt-route with preflight (axhub-related)",
    subcommand: "prompt-route",
    thresholdMs: 120,
    env: { AXHUB_BIN: fakeAxhub },
    payload: { hook_event_name: "UserPromptSubmit", prompt: "axhub 배포해" },
    assertOutput(stdout: string): void {
      if (stdout.includes("skills/")) throw new Error(`Approach E: no skill path, got ${stdout}`);
      const parsed = JSON.parse(stdout);
      const ctx = parsed?.hookSpecificOutput?.additionalContext;
      if (typeof ctx !== "string" || !ctx.includes("<axhub-preflight-status>")) {
        throw new Error(`expected preflight context, got ${stdout}`);
      }
    },
  },
  {
    name: "prompt-route with preflight (off-topic)",
    subcommand: "prompt-route",
    thresholdMs: 120,
    env: { AXHUB_BIN: fakeAxhub },
    payload: { hook_event_name: "UserPromptSubmit", prompt: "오늘 날씨 알려줘" },
    assertOutput(stdout: string): void {
      // Approach E: same preflight context regardless of intent.
      if (stdout.includes("skills/")) throw new Error(`Approach E: no skill path, got ${stdout}`);
      const parsed = JSON.parse(stdout);
      const ctx = parsed?.hookSpecificOutput?.additionalContext;
      if (typeof ctx !== "string") throw new Error(`expected preflight string context, got ${stdout}`);
    },
  },
  {
    name: "cumulative PreToolUse Bash chain",
    subcommand: "preauth-check+commit-gate",
    thresholdMs: 90,
    payload: {
      hook_event_name: "PreToolUse",
      tool_name: "Bash",
      tool_input: { command: "echo hello" },
    },
    assertOutput(stdout: string): void {
      const parsed = JSON.parse(stdout.split("\n").filter(Boolean).at(-1) ?? "{}");
      const decision = parsed?.hookSpecificOutput?.permissionDecision;
      if (decision !== "allow") throw new Error(`expected cumulative allow, got ${stdout}`);
    },
  },
  {
    name: "cumulative PostToolUse Bash chain",
    subcommand: "classify-exit+test-classifier",
    thresholdMs: 60,
    payload: {
      hook_event_name: "PostToolUse",
      tool_name: "Bash",
      tool_input: { command: "echo hello" },
      tool_response: { exit_code: 0, stdout: "hello\n", stderr: "" },
    },
    assertOutput(stdout: string): void {
      const lines = stdout.split("\n").filter(Boolean);
      if (lines.some((line) => JSON.stringify(JSON.parse(line)) !== "{}")) {
        throw new Error(`expected cumulative no-op {}, got ${stdout}`);
      }
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
    const helperPath: string = helper ?? fail("helper binary missing");
    const subcommands = scenario.subcommand.split("+");
    const outputs: string[] = [];
    let failed: { status: number | null; stderr: string } | null = null;
    for (const subcommand of subcommands) {
      const result = spawnSync(helperPath, [subcommand], {
        cwd: REPO_ROOT,
        input,
        encoding: "utf8",
        env: { ...process.env, AXHUB_TELEMETRY: "0", ...(scenario as { env?: Record<string, string> }).env },
      });
      if (result.status !== 0) {
        failed = { status: result.status, stderr: result.stderr };
        break;
      }
      outputs.push(result.stdout.trim());
    }
    const elapsedMs = Number(process.hrtime.bigint() - started) / 1_000_000;

    if (failed) {
      fail(`${scenario.name}: exit ${failed.status}; stderr=${failed.stderr.trim()}`);
    }
    try {
      scenario.assertOutput(outputs.join("\n"));
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
