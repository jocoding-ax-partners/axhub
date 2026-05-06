#!/usr/bin/env bun
import { createHash } from "node:crypto";
import { spawnSync, type SpawnSyncReturns } from "node:child_process";
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";

export const REPO_ROOT = join(import.meta.dir, "..");
export const DEFAULT_FIXTURE_APP = join(REPO_ROOT, "tests/e2e/fixtures/vibe-static-app");
export const DEFAULT_HELPER = join(REPO_ROOT, "bin/axhub-helpers");
export const FORBIDDEN_ARTIFACT_KEYS = new Set([
  "token",
  "access_token",
  "refresh_token",
  "authorization",
  "email",
  "user_email",
  "user_name",
  "app_slug",
  "slug",
  "app_name",
  "subdomain",
  "deploy_url",
  "live_url",
  "url",
  "command",
  "command_argv",
  "argv",
  "stdout",
  "stderr",
  "response",
  "response_body",
  "branch",
  "commit",
  "commit_sha",
]);

export type CleanupMode = "preprovisioned" | "ttl";
export type GateMode = "advisory" | "blocking";

export type MeasurementConfig = {
  token: string;
  endpoint: string;
  destructive: true;
  maxRuns: number;
  costBudgetUsd: number;
  cleanupMode: CleanupMode;
  fixtureApp: string;
  helperPath: string;
  outputPath?: string;
  watchTimeout: string;
};

export type TranscriptMetrics = {
  ask_count: number;
  consent_mint_count: number;
  consent_block_count: number;
};

export type MeasurementRun = {
  run_id: string;
  success: boolean;
  failure_code?: string;
  phase_durations_ms: Record<string, number>;
  total_seconds: number;
  ask_count: number;
  consent_mint_count: number;
  consent_block_count: number;
  readiness_source?: string;
  live_url_present: boolean;
  live_url_host_hash?: string;
  cleanup_mode: CleanupMode;
};

export type MeasurementSummary = {
  schema_version: "vibe-bootstrap-measurement/v1";
  runs: MeasurementRun[];
  success_count: number;
  failure_count: number;
  ask_count: number;
  consent_block_count: number;
  consent_mint_count: number;
  phase_durations_ms: Record<string, number[]>;
  total_seconds: number[];
  p50_seconds: number;
  p95_seconds: number;
  sample_size: number;
  endpoint_class: string;
  readiness_source: string;
  live_url_present: boolean;
  cleanup_mode: CleanupMode;
  started_at: string;
  completed_at: string;
};

export class MeasurementConfigError extends Error {
  constructor(public readonly code: string, message: string) {
    super(message);
    this.name = "MeasurementConfigError";
  }
}

const asPositiveInt = (value: string | undefined, name: string): number => {
  if (!value) throw new MeasurementConfigError(`${name}.missing`, `${name} is required`);
  if (!/^\d+$/.test(value)) throw new MeasurementConfigError(`${name}.invalid`, `${name} must be a positive integer`);
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed <= 0) {
    throw new MeasurementConfigError(`${name}.invalid`, `${name} must be a positive integer`);
  }
  return parsed;
};

const asPositiveNumber = (value: string | undefined, name: string): number => {
  if (!value) throw new MeasurementConfigError(`${name}.missing`, `${name} is required`);
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    throw new MeasurementConfigError(`${name}.invalid`, `${name} must be a positive number`);
  }
  return parsed;
};

export function validateMeasurementEnv(env: NodeJS.ProcessEnv = process.env): MeasurementConfig {
  const token = env["AXHUB_E2E_STAGING_TOKEN"];
  if (!token) throw new MeasurementConfigError("token.missing", "AXHUB_E2E_STAGING_TOKEN is required");
  const endpoint = env["AXHUB_E2E_STAGING_ENDPOINT"];
  if (!endpoint) throw new MeasurementConfigError("endpoint.missing", "AXHUB_E2E_STAGING_ENDPOINT is required; default endpoint fallback is refused");
  if (!/^https?:\/\//.test(endpoint)) {
    throw new MeasurementConfigError("endpoint.invalid", "AXHUB_E2E_STAGING_ENDPOINT must be an absolute http(s) URL");
  }
  if (env["AXHUB_E2E_DESTRUCTIVE"] !== "1") {
    throw new MeasurementConfigError("destructive_opt_in.missing", "AXHUB_E2E_DESTRUCTIVE=1 is required for full-chain measurement");
  }
  const maxRuns = asPositiveInt(env["AXHUB_E2E_MAX_RUNS"], "AXHUB_E2E_MAX_RUNS");
  const costBudgetUsd = asPositiveNumber(env["AXHUB_E2E_COST_BUDGET_USD"], "AXHUB_E2E_COST_BUDGET_USD");
  const cleanupMode = env["AXHUB_E2E_CLEANUP_MODE"];
  if (cleanupMode !== "preprovisioned" && cleanupMode !== "ttl") {
    throw new MeasurementConfigError("cleanup_mode.missing", "AXHUB_E2E_CLEANUP_MODE must be preprovisioned or ttl");
  }
  const fixtureApp = resolve(env["AXHUB_E2E_FIXTURE_APP"] ?? DEFAULT_FIXTURE_APP);
  if (!existsSync(fixtureApp)) {
    throw new MeasurementConfigError("fixture_app.missing", `fixture app missing: ${fixtureApp}`);
  }
  const helperPath = resolve(env["AXHUB_E2E_HELPER_PATH"] ?? DEFAULT_HELPER);
  if (!existsSync(helperPath)) {
    throw new MeasurementConfigError("helper.missing", `helper binary missing: ${helperPath}; run bun run build first`);
  }
  return {
    token,
    endpoint,
    destructive: true,
    maxRuns,
    costBudgetUsd,
    cleanupMode,
    fixtureApp,
    helperPath,
    outputPath: env["AXHUB_E2E_MEASUREMENT_OUT"],
    watchTimeout: env["AXHUB_E2E_WATCH_TIMEOUT"] ?? "10m",
  };
}

export function percentile(values: number[], p: number): number {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const index = Math.min(sorted.length - 1, Math.max(0, Math.ceil(sorted.length * p) - 1));
  return sorted[index] ?? 0;
}

export function parseTranscriptMetrics(lines: string[]): TranscriptMetrics {
  const metrics: TranscriptMetrics = { ask_count: 0, consent_mint_count: 0, consent_block_count: 0 };
  for (const line of lines) {
    if (!line.trim()) continue;
    let value: unknown;
    try {
      value = JSON.parse(line);
    } catch {
      continue;
    }
    const event = readString(value, ["/event", "/hook_event_name", "/hookSpecificOutput/hookEventName"]);
    const decision = readString(value, ["/hookSpecificOutput/permissionDecision", "/permissionDecision"]);
    if (event === "AskUserQuestion" || event === "ask_user_question") metrics.ask_count += 1;
    if (event === "consent_mint") metrics.consent_mint_count += 1;
    if (event === "preauth_check_deny" || decision === "deny") metrics.consent_block_count += 1;
  }
  return metrics;
}

export function verifyReadinessStatus(value: unknown): { ok: boolean; source: string; reason?: string } {
  const status = readString(value, ["/status", "/deployment/status", "/data/status", "/state"]);
  if (!status) return { ok: false, source: "deploy_status", reason: "status_missing" };
  const normalized = status.toLowerCase();
  if (["succeeded", "success", "ready", "deployed", "live"].includes(normalized)) {
    return { ok: true, source: "deploy_status" };
  }
  return { ok: false, source: "deploy_status", reason: `status_${normalized}` };
}

export function verifyOpenUrlPresence(value: unknown): { live_url_present: boolean; live_url_host_hash?: string; reason?: string } {
  const url = findUrl(value);
  if (!url) return { live_url_present: false, reason: "url_missing" };
  try {
    const parsed = new URL(url);
    return {
      live_url_present: true,
      live_url_host_hash: createHash("sha256").update(parsed.host).digest("hex").slice(0, 16),
    };
  } catch {
    return { live_url_present: false, reason: "url_invalid" };
  }
}

export function buildMeasurementSummary(runs: MeasurementRun[], endpoint: string, cleanupMode: CleanupMode, startedAt: string, completedAt: string): MeasurementSummary {
  const successful = runs.filter((run) => run.success);
  const phaseNames = new Set(runs.flatMap((run) => Object.keys(run.phase_durations_ms)));
  const phaseDurations: Record<string, number[]> = {};
  for (const phase of phaseNames) {
    phaseDurations[phase] = runs.map((run) => run.phase_durations_ms[phase]).filter((v): v is number => Number.isFinite(v));
  }
  const totals = successful.map((run) => run.total_seconds);
  return sanitizeArtifact({
    schema_version: "vibe-bootstrap-measurement/v1",
    runs,
    success_count: successful.length,
    failure_count: runs.length - successful.length,
    ask_count: runs.reduce((sum, run) => sum + run.ask_count, 0),
    consent_block_count: runs.reduce((sum, run) => sum + run.consent_block_count, 0),
    consent_mint_count: runs.reduce((sum, run) => sum + run.consent_mint_count, 0),
    phase_durations_ms: phaseDurations,
    total_seconds: totals,
    p50_seconds: percentile(totals, 0.50),
    p95_seconds: percentile(totals, 0.95),
    sample_size: totals.length,
    endpoint_class: classifyEndpoint(endpoint),
    readiness_source: runs.find((run) => run.readiness_source)?.readiness_source ?? "unknown",
    live_url_present: successful.length > 0 && successful.every((run) => run.live_url_present),
    cleanup_mode: cleanupMode,
    started_at: startedAt,
    completed_at: completedAt,
  }) as MeasurementSummary;
}

export function sanitizeArtifact(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sanitizeArtifact);
  if (value && typeof value === "object") {
    const out: Record<string, unknown> = {};
    for (const [key, nested] of Object.entries(value)) {
      if (FORBIDDEN_ARTIFACT_KEYS.has(key.toLowerCase())) continue;
      out[key] = sanitizeArtifact(nested);
    }
    return out;
  }
  if (typeof value === "string") {
    if (/axhub_(pat|tok)_[A-Za-z0-9_.-]+/.test(value)) return "[redacted-token]";
    if (/Bearer\s+[A-Za-z0-9_.-]+/.test(value)) return "Bearer [redacted]";
    if (/https?:\/\//.test(value)) return "[redacted-url]";
    if (/[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}/i.test(value)) return "[redacted-email]";
  }
  return value;
}

export function artifactContainsForbiddenValue(value: unknown): boolean {
  const text = JSON.stringify(value);
  return /axhub_(pat|tok)_|Bearer\s+|https?:\/\/|[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}/i.test(text);
}

export function classifyEndpoint(endpoint: string): string {
  try {
    const host = new URL(endpoint).host;
    if (/staging/i.test(host)) return "staging";
    if (/localhost|127\.0\.0\.1/.test(host)) return "local";
    return "explicit";
  } catch {
    return "invalid";
  }
}

type Runner = (cmd: string[], opts: { cwd: string; env: NodeJS.ProcessEnv; input?: string }) => SpawnSyncReturns<string>;

const defaultRunner: Runner = (cmd, opts) => spawnSync(cmd[0] ?? "", cmd.slice(1), {
  cwd: opts.cwd,
  env: opts.env,
  input: opts.input,
  encoding: "utf8",
});

export function runLiveMeasurement(config: MeasurementConfig, runner: Runner = defaultRunner): MeasurementSummary {
  const startedAt = new Date().toISOString();
  const runs: MeasurementRun[] = [];
  for (let i = 0; i < config.maxRuns; i += 1) {
    runs.push(runOneMeasurement(config, i + 1, runner));
  }
  return buildMeasurementSummary(runs, config.endpoint, config.cleanupMode, startedAt, new Date().toISOString());
}

function runOneMeasurement(config: MeasurementConfig, index: number, runner: Runner): MeasurementRun {
  const runId = `s4-measure-${Date.now()}-${index}`;
  const sandbox = mkdtempSync(join(tmpdir(), `${runId}-`));
  const projectDir = join(sandbox, basename(config.fixtureApp));
  const stateHome = join(sandbox, "state");
  const phaseStarts = new Map<string, bigint>();
  const phaseDurations: Record<string, number> = {};
  const transcript: string[] = [];
  const runStarted = process.hrtime.bigint();

  const env: NodeJS.ProcessEnv = {
    ...process.env,
    AXHUB_TOKEN: config.token,
    AXHUB_ENDPOINT: config.endpoint,
    AXHUB_TELEMETRY: "1",
    XDG_STATE_HOME: stateHome,
    CLAUDE_NON_INTERACTIVE: "1",
  };

  const startPhase = (phase: string): void => {
    phaseStarts.set(phase, process.hrtime.bigint());
  };
  const endPhase = (phase: string): void => {
    const started = phaseStarts.get(phase);
    if (!started) return;
    phaseDurations[phase] = Number(process.hrtime.bigint() - started) / 1_000_000;
  };

  try {
    mkdirSync(stateHome, { recursive: true });
    cpSync(config.fixtureApp, projectDir, { recursive: true });

    let readinessSource = "unknown";
    let liveUrlPresent = false;
    let liveUrlHostHash: string | undefined;

    for (let step = 0; step < 20; step += 1) {
      startPhase("bootstrap_plan");
      const planned = runner([config.helperPath, "bootstrap", "--auto-chain", "--json"], { cwd: projectDir, env });
      endPhase("bootstrap_plan");
      const plan = parseJson(planned.stdout, "bootstrap_plan_stdout");
      const state = String(plan["state"] ?? "");
      const nextAction = typeof plan["next_action"] === "string" ? plan["next_action"] : undefined;
      const command = Array.isArray(plan["command"]) ? plan["command"].map(String) : undefined;

      if (state === "deploying") {
        const stateFile = parseJson(readFileSync(join(projectDir, ".axhub/bootstrap.state.json"), "utf8"), "bootstrap_state");
        const deployId = String(stateFile["last_deploy_id"] ?? "");
        const app = String(stateFile["app_id"] ?? stateFile["app_slug"] ?? "");
        if (!deployId || !app) throw new MeasurementConfigError("readiness.input_missing", "deploying state lacks deployment/app id");

        startPhase("deploy_watch");
        const status = runner(["axhub", "deploy", "status", deployId, "--app", app, "--watch", "--watch-timeout", config.watchTimeout, "--json"], { cwd: projectDir, env });
        endPhase("deploy_watch");
        const statusJson = parseLastJson(status.stdout, "deploy_status_stdout");
        const readiness = verifyReadinessStatus(statusJson);
        readinessSource = readiness.source;
        if (!readiness.ok) throw new MeasurementConfigError("readiness.not_ready", readiness.reason ?? "deploy not ready");

        startPhase("open_url");
        const open = runner(["axhub", "open", app, "--json"], { cwd: projectDir, env });
        endPhase("open_url");
        const openJson = parseJson(open.stdout, "open_stdout");
        const openResult = verifyOpenUrlPresence(openJson);
        if (!openResult.live_url_present) throw new MeasurementConfigError("live_url.missing", openResult.reason ?? "live URL missing");
        liveUrlPresent = true;
        liveUrlHostHash = openResult.live_url_host_hash;
        const metrics = parseTranscriptMetrics(transcript);
        return {
          run_id: runId,
          success: true,
          phase_durations_ms: phaseDurations,
          total_seconds: Number(process.hrtime.bigint() - runStarted) / 1_000_000_000,
          ...metrics,
          readiness_source: readinessSource,
          live_url_present: liveUrlPresent,
          live_url_host_hash: liveUrlHostHash,
          cleanup_mode: config.cleanupMode,
        };
      }

      if (!command || !nextAction) throw new MeasurementConfigError("bootstrap.typed_blocker", `bootstrap stopped at ${state}`);

      if (command[0] === "git") {
        if (nextAction === "first_commit") {
          runner(["git", "config", "user.email", "s4-measure@example.invalid"], { cwd: projectDir, env });
          runner(["git", "config", "user.name", "S4 Measure"], { cwd: projectDir, env });
          runner(["git", "add", "."], { cwd: projectDir, env });
        }
        startPhase(nextAction);
        const result = runner(command, { cwd: projectDir, env });
        endPhase(nextAction);
        if (result.status !== 0) throw new MeasurementConfigError(`${nextAction}.failed`, result.stderr || result.stdout);
        continue;
      }

      if (command[0] === "axhub" && (nextAction === "apps_create" || nextAction === "deploy_create")) {
        transcript.push(JSON.stringify({ event: "consent_mint" }));
        startPhase(nextAction);
        const result = runner(command, { cwd: projectDir, env });
        endPhase(nextAction);
        const stdoutJson = parseJson(result.stdout || "{}", `${nextAction}_stdout`);
        const envelope = {
          schema_version: "bootstrap-record/v1",
          pending_action_id: plan["pending_action_id"],
          pending_action_hash: plan["pending_action_hash"],
          command_argv: command,
          exit_code: result.status ?? 1,
          stdout_json: stdoutJson,
          stderr: "",
        };
        startPhase(`${nextAction}_record`);
        const recorded = runner([config.helperPath, "bootstrap", "--record", nextAction, "--json"], {
          cwd: projectDir,
          env,
          input: JSON.stringify(envelope),
        });
        endPhase(`${nextAction}_record`);
        if (recorded.status !== 0) throw new MeasurementConfigError(`${nextAction}.record_failed`, recorded.stdout || recorded.stderr);
        continue;
      }

      throw new MeasurementConfigError("command.unsupported", `unsupported planned command class: ${command[0] ?? "missing"}`);
    }
    throw new MeasurementConfigError("bootstrap.max_steps", "bootstrap did not reach deploying within 20 steps");
  } catch (error) {
    const metrics = parseTranscriptMetrics(transcript);
    return {
      run_id: runId,
      success: false,
      failure_code: error instanceof MeasurementConfigError ? error.code : "measurement.failed",
      phase_durations_ms: phaseDurations,
      total_seconds: Number(process.hrtime.bigint() - runStarted) / 1_000_000_000,
      ...metrics,
      live_url_present: false,
      cleanup_mode: config.cleanupMode,
    };
  } finally {
    rmSync(sandbox, { recursive: true, force: true });
  }
}

function readString(value: unknown, pointers: string[]): string | undefined {
  for (const pointer of pointers) {
    const found = readPointer(value, pointer);
    if (typeof found === "string" && found.length > 0) return found;
  }
  return undefined;
}

function readPointer(value: unknown, pointer: string): unknown {
  const parts = pointer.split("/").slice(1).map((part) => part.replace(/~1/g, "/").replace(/~0/g, "~"));
  let current = value as unknown;
  for (const part of parts) {
    if (!current || typeof current !== "object") return undefined;
    current = (current as Record<string, unknown>)[part];
  }
  return current;
}

function findUrl(value: unknown): string | undefined {
  if (typeof value === "string" && /^https?:\/\//.test(value)) return value;
  if (Array.isArray(value)) {
    for (const item of value) {
      const found = findUrl(item);
      if (found) return found;
    }
  }
  if (value && typeof value === "object") {
    for (const [key, nested] of Object.entries(value)) {
      if (/url$/i.test(key) || /^(url|deploy_url|public_url|live_url)$/i.test(key)) {
        const found = findUrl(nested);
        if (found) return found;
      }
    }
  }
  return undefined;
}

function parseJson(raw: string, label: string): Record<string, unknown> {
  try {
    return JSON.parse(raw) as Record<string, unknown>;
  } catch (error) {
    throw new MeasurementConfigError(`${label}.invalid_json`, error instanceof Error ? error.message : String(error));
  }
}

function parseLastJson(raw: string, label: string): Record<string, unknown> {
  const lines = raw.trim().split(/\r?\n/).filter(Boolean);
  return parseJson(lines[lines.length - 1] ?? raw, label);
}

function printUsageAndExit(): never {
  process.stderr.write(`Usage: bun scripts/measure-vibe-bootstrap.ts [--out measurement-summary.json]\n\nRequired env for live measurement:\n  AXHUB_E2E_STAGING_TOKEN\n  AXHUB_E2E_STAGING_ENDPOINT\n  AXHUB_E2E_DESTRUCTIVE=1\n  AXHUB_E2E_MAX_RUNS=<positive integer>\n  AXHUB_E2E_COST_BUDGET_USD=<positive number>\n  AXHUB_E2E_CLEANUP_MODE=preprovisioned|ttl\n`);
  process.exit(64);
}

if (import.meta.main) {
  const outIndex = process.argv.indexOf("--out");
  if (process.argv.includes("--help")) printUsageAndExit();
  const outPath = outIndex >= 0 ? process.argv[outIndex + 1] : undefined;
  if (outIndex >= 0 && !outPath) printUsageAndExit();
  try {
    const config = validateMeasurementEnv({ ...process.env, ...(outPath ? { AXHUB_E2E_MEASUREMENT_OUT: outPath } : {}) });
    const summary = runLiveMeasurement(config);
    const json = `${JSON.stringify(summary, null, 2)}\n`;
    if (config.outputPath) {
      writeFileSync(config.outputPath, json);
    } else {
      process.stdout.write(json);
    }
    process.exit(summary.failure_count === 0 ? 0 : 1);
  } catch (error) {
    const code = error instanceof MeasurementConfigError ? error.code : "measurement.unhandled";
    const message = error instanceof Error ? error.message : String(error);
    process.stderr.write(JSON.stringify({ error_code: code, message }) + "\n");
    process.exit(64);
  }
}
