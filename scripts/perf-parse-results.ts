/**
 * Phase 0 perf gate parser.
 *
 * Spec: .plan/deploy-time-reduction/phase-0-measurement-first.md §8
 *
 * Reads test-results.json (produced by tests/perf/deploy_walltime.test.ts),
 * compares each scenario's p95 against the published ceiling, prints a
 * one-line summary per scenario, and exits non-zero if any scenario
 * exceeds its ceiling.
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

export const DEFAULT_CEILINGS: Record<string, number> = {
  "scenario-1-warm-redeploy": 60_000,
  "scenario-2-cold-first-deploy": 90_000,
  "scenario-3-fresh-home-no-token": 120_000,
};

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

export function format(checks: CeilingCheck[]): string {
  return checks
    .map((c) => {
      const verdict = c.ok ? "PASS" : "FAIL";
      const p95 = c.p95 < 0 ? "missing" : `${c.p95}ms`;
      return `${verdict} ${c.scenarioId} p95=${p95} limit=${c.limit}ms`;
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
  const checks = evaluate(parsed);
  process.stdout.write(format(checks) + "\n");
  const failed = checks.filter((c) => !c.ok);
  process.exit(failed.length === 0 ? 0 : 1);
}

if (import.meta.main) {
  await main();
}
