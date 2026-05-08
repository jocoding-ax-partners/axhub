import { describe, expect, test } from "bun:test";
import {
  DEFAULT_CEILINGS,
  evaluate,
  format,
  type ResultsFile,
} from "../scripts/perf-parse-results";

const passResults: ResultsFile = {
  "scenario-1-warm-redeploy": { avg: 30, p95: 60, runs: [] },
  "scenario-2-cold-first-deploy": { avg: 100, p95: 200, runs: [] },
  "scenario-3-fresh-home-no-token": { avg: 200, p95: 350, runs: [] },
};

const failResults: ResultsFile = {
  "scenario-1-warm-redeploy": { avg: 30, p95: 100_000, runs: [] },
  "scenario-2-cold-first-deploy": { avg: 100, p95: 200, runs: [] },
  "scenario-3-fresh-home-no-token": { avg: 200, p95: 350, runs: [] },
};

describe("perf-parse-results.evaluate", () => {
  test("all scenarios under ceiling → all ok", () => {
    const checks = evaluate(passResults);
    expect(checks).toHaveLength(3);
    expect(checks.every((c) => c.ok)).toBe(true);
  });

  test("scenario over ceiling → marks that scenario failed", () => {
    const checks = evaluate(failResults);
    const warm = checks.find((c) => c.scenarioId === "scenario-1-warm-redeploy");
    expect(warm?.ok).toBe(false);
    expect(warm?.limit).toBe(DEFAULT_CEILINGS["scenario-1-warm-redeploy"]);
  });

  test("missing scenario → ok=false with p95=-1", () => {
    const checks = evaluate({});
    expect(checks.every((c) => !c.ok)).toBe(true);
    expect(checks.every((c) => c.p95 === -1)).toBe(true);
  });

  test("format prints PASS/FAIL line per scenario", () => {
    const out = format(evaluate(failResults));
    expect(out).toContain("FAIL scenario-1-warm-redeploy");
    expect(out).toContain("PASS scenario-2-cold-first-deploy");
  });
});
