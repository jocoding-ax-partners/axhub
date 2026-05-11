/**
 * Phase 0 perf gate parser.
 *
 * Spec: .plan/deploy-time-reduction/phase-0-measurement-first.md §8
 *
 * Reads test-results.json (produced by tests/perf/deploy_walltime.test.ts),
 * compares each scenario's p95 against:
 *   1. the published absolute ceiling, and
 *   2. the committed Phase 0 baseline for this CI OS with a max +5% delta.
 *
 * It prints a one-line summary per scenario/check and exits non-zero if any
 * scenario exceeds either gate.
 *
 * Ceilings come from the canonical plan (1.5× current baseline). They are
 * intentionally generous so that Phase 0 measurement can land without
 * breaking CI on existing variance; Phase 1+ tightens these as
 * optimizations land.
 */

import { readFile } from "node:fs/promises";
import process from "node:process";

export interface ScenarioRecord {
  avg: number;
  p95: number;
  runs: { walltime_ms: number; exit_code: number }[];
  scope_note?: string;
}

export type ResultsFile = Record<string, ScenarioRecord>;

export interface CeilingCheck {
  scenarioId: string;
  p95: number;
  limit: number;
  ok: boolean;
}

export interface BaselineRecord {
  avg: number;
  p95: number;
  runs: number;
}

export type BaselineFile = Record<string, BaselineRecord>;

export interface BaselineMatrix {
  baselines: Record<string, BaselineFile>;
  max_delta_ratio?: number;
  generated_from?: string;
}

export interface DeltaCheck {
  scenarioId: string;
  p95: number;
  baselineP95: number;
  maxAllowedP95: number;
  deltaPct: number;
  ok: boolean;
}

export const DEFAULT_CEILINGS: Record<string, number> = {
  "scenario-1-warm-redeploy": 60_000,
  "scenario-2-cold-first-deploy": 90_000,
  "scenario-3-fresh-home-no-token": 120_000,
};

export const DEFAULT_MAX_DELTA_RATIO = 0.05;

export const SCENARIO_LABELS: Record<string, string> = {
  "scenario-1-warm-redeploy": "Scenario 1 — warm redeploy",
  "scenario-2-cold-first-deploy": "Scenario 2 — cold first deploy",
  "scenario-3-fresh-home-no-token": "Scenario 3 — fresh HOME (no token)",
};

const LABEL_TO_SCENARIO = new Map(
  Object.entries(SCENARIO_LABELS).map(([scenarioId, label]) => [label, scenarioId]),
);

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function parseMsCell(value: string): number | null {
  const match = value.match(/^(\d+)ms$/);
  if (!match) return null;
  return Number(match[1]);
}

function parseRunsCell(value: string): number | null {
  const match = value.match(/^(\d+)$/);
  if (!match) return null;
  return Number(match[1]);
}

export function normalizeBaselineOs(value: string | undefined): string {
  if (!value) return "ubuntu-latest";
  const lower = value.toLowerCase();
  if (lower === "linux") return "ubuntu-latest";
  if (lower === "macos" || lower === "mac") return "macos-latest";
  if (lower === "windows" || lower === "win32") return "windows-latest";
  return value;
}

export function parseBaselineMarkdown(markdown: string, os: string): BaselineFile {
  const heading = new RegExp(`^###\\s+${escapeRegExp(os)}\\s*$`, "m");
  const match = heading.exec(markdown);
  if (!match) return {};

  const afterHeading = markdown.slice(match.index + match[0].length);
  const nextHeading = afterHeading.search(/^###\s+/m);
  const section =
    nextHeading >= 0 ? afterHeading.slice(0, nextHeading) : afterHeading;
  const baselines: BaselineFile = {};

  for (const line of section.split("\n")) {
    if (!line.startsWith("|")) continue;
    const cells = line
      .split("|")
      .map((cell) => cell.trim())
      .filter((cell) => cell.length > 0);
    if (cells.length !== 4) continue;
    const [label, avgCell, p95Cell, runsCell] = cells;
    const scenarioId = LABEL_TO_SCENARIO.get(label);
    if (!scenarioId) continue;
    const avg = parseMsCell(avgCell);
    const p95 = parseMsCell(p95Cell);
    const runs = parseRunsCell(runsCell);
    if (avg === null || p95 === null || runs === null) continue;
    baselines[scenarioId] = { avg, p95, runs };
  }

  return baselines;
}

export function parseBaselineJson(raw: string, os: string): BaselineFile {
  const parsed = JSON.parse(raw) as Partial<BaselineMatrix> &
    Record<string, BaselineFile | undefined>;
  if (parsed.baselines && typeof parsed.baselines === "object") {
    return (parsed.baselines as Record<string, BaselineFile>)[os] ?? {};
  }
  return (parsed as Record<string, BaselineFile>)[os] ?? {};
}

export function parseBaseline(raw: string, os: string, path = ""): BaselineFile {
  const trimmed = raw.trimStart();
  if (path.endsWith(".json") || trimmed.startsWith("{")) {
    return parseBaselineJson(raw, os);
  }
  return parseBaselineMarkdown(raw, os);
}

export function evaluate(
  results: ResultsFile,
  ceilings: Record<string, number> = DEFAULT_CEILINGS,
): CeilingCheck[] {
  return Object.keys(ceilings).map((scenarioId) => {
    const record = results[scenarioId];
    const limit = ceilings[scenarioId];
    if (!record) {
      return { scenarioId, p95: -1, limit, ok: false };
    }
    return {
      scenarioId,
      p95: record.p95,
      limit,
      ok: record.p95 <= limit,
    };
  });
}

export function evaluateDeltas(
  results: ResultsFile,
  baselines: BaselineFile,
  maxDeltaRatio = DEFAULT_MAX_DELTA_RATIO,
): DeltaCheck[] {
  return Object.keys(SCENARIO_LABELS).map((scenarioId) => {
    const record = results[scenarioId];
    const baseline = baselines[scenarioId];
    if (!record || !baseline || baseline.p95 <= 0) {
      return {
        scenarioId,
        p95: record?.p95 ?? -1,
        baselineP95: baseline?.p95 ?? -1,
        maxAllowedP95: -1,
        deltaPct: Number.POSITIVE_INFINITY,
        ok: false,
      };
    }
    const maxAllowedP95 = baseline.p95 * (1 + maxDeltaRatio);
    const deltaPct = ((record.p95 - baseline.p95) / baseline.p95) * 100;
    return {
      scenarioId,
      p95: record.p95,
      baselineP95: baseline.p95,
      maxAllowedP95,
      deltaPct,
      ok: record.p95 <= maxAllowedP95,
    };
  });
}

export function format(checks: CeilingCheck[]): string {
  return checks
    .map((c) => {
      const verdict = c.ok ? "PASS" : "FAIL";
      const p95 = c.p95 < 0 ? "missing" : `${c.p95}ms`;
      return `${verdict} ${c.scenarioId} p95=${p95} limit=${c.limit}ms`;
    })
    .join("\n");
}

export function formatDeltas(checks: DeltaCheck[], os: string): string {
  return checks
    .map((c) => {
      const verdict = c.ok ? "PASS" : "FAIL";
      const p95 = c.p95 < 0 ? "missing" : `${c.p95}ms`;
      const baseline =
        c.baselineP95 < 0 ? "missing" : `${c.baselineP95}ms`;
      const limit =
        c.maxAllowedP95 < 0 ? "missing" : `${Math.floor(c.maxAllowedP95)}ms`;
      const delta = Number.isFinite(c.deltaPct)
        ? `${c.deltaPct.toFixed(2)}%`
        : "missing";
      return `${verdict} ${c.scenarioId} ${os} p95=${p95} baseline=${baseline} max_delta_p95=${limit} delta=${delta}`;
    })
    .join("\n");
}

async function main(): Promise<void> {
  const path = process.env.RESULTS_FILE ?? "test-results.json";
  let raw: string;
  try {
    raw = await readFile(path, "utf8");
  } catch (err) {
    console.error(`perf-parse: cannot read ${path}: ${(err as Error).message}`);
    process.exit(1);
  }
  let parsed: ResultsFile;
  try {
    parsed = JSON.parse(raw) as ResultsFile;
  } catch (err) {
    console.error(`perf-parse: invalid JSON in ${path}: ${(err as Error).message}`);
    process.exit(1);
  }
  const ceilingChecks = evaluate(parsed);

  const baselinePath =
    process.env.BASELINE_FILE ?? "tests/fixtures/perf/phase-0-baselines.json";
  const baselineOs = normalizeBaselineOs(
    process.env.PERF_BASELINE_OS ?? process.env.RUNNER_OS,
  );
  let baselineRaw: string;
  try {
    baselineRaw = await readFile(baselinePath, "utf8");
  } catch (err) {
    console.error(
      `perf-parse: cannot read baseline ${baselinePath}: ${(err as Error).message}`,
    );
    process.exit(1);
  }
  const baselines = parseBaseline(baselineRaw, baselineOs, baselinePath);
  const deltaChecks = evaluateDeltas(
    parsed,
    baselines,
    Number(process.env.PERF_MAX_DELTA_RATIO ?? DEFAULT_MAX_DELTA_RATIO),
  );

  process.stdout.write(`${format(ceilingChecks)}\n${formatDeltas(deltaChecks, baselineOs)}\n`);
  const failed = [
    ...ceilingChecks.filter((c) => !c.ok),
    ...deltaChecks.filter((c) => !c.ok),
  ];
  process.exit(failed.length === 0 ? 0 : 1);
}

if (import.meta.main) {
  await main();
}
