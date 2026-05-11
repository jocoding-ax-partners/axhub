import { describe, expect, test } from "bun:test";
import {
  DEFAULT_CEILINGS,
  evaluateDeltas,
  evaluate,
  format,
  formatDeltas,
  normalizeBaselineOs,
  parseBaseline,
  parseBaselineJson,
  parseBaselineMarkdown,
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

const baselineMarkdown = `
### ubuntu-latest

| Scenario | avg | p95 | runs |
|----------|-----|-----|------|
| Scenario 1 — warm redeploy | 1000ms | 1000ms | 5 |
| Scenario 2 — cold first deploy | 2000ms | 2000ms | 5 |
| Scenario 3 — fresh HOME (no token) | 3000ms | 3000ms | 5 |

### macos-latest

_pending CI run_
`;

const baselineJson = JSON.stringify({
  baselines: {
    "windows-latest": {
      "scenario-1-warm-redeploy": { avg: 1100, p95: 1200, runs: 5 },
      "scenario-2-cold-first-deploy": { avg: 2100, p95: 2200, runs: 5 },
      "scenario-3-fresh-home-no-token": { avg: 3100, p95: 3200, runs: 5 },
    },
  },
});

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

describe("perf-parse-results Phase 0 delta gate", () => {
  test("parses OS-specific baseline rows from MEASUREMENTS.md", () => {
    const baselines = parseBaselineMarkdown(baselineMarkdown, "ubuntu-latest");
    expect(baselines["scenario-1-warm-redeploy"].p95).toBe(1000);
    expect(baselines["scenario-2-cold-first-deploy"].avg).toBe(2000);
    expect(baselines["scenario-3-fresh-home-no-token"].runs).toBe(5);
  });

  test("parses tracked JSON baselines for CI", () => {
    const baselines = parseBaselineJson(baselineJson, "windows-latest");
    expect(baselines["scenario-1-warm-redeploy"].p95).toBe(1200);
    expect(parseBaseline(baselineJson, "windows-latest", "baseline.json")).toEqual(
      baselines,
    );
  });

  test("missing or pending OS baseline fails closed", () => {
    const checks = evaluateDeltas(
      passResults,
      parseBaselineMarkdown(baselineMarkdown, "macos-latest"),
    );
    expect(checks.every((c) => !c.ok)).toBe(true);
    expect(formatDeltas(checks, "macos-latest")).toContain("baseline=missing");
  });

  test("allows p95 up to five percent over baseline", () => {
    const checks = evaluateDeltas(
      {
        "scenario-1-warm-redeploy": { avg: 1000, p95: 1050, runs: [] },
        "scenario-2-cold-first-deploy": { avg: 2000, p95: 2100, runs: [] },
        "scenario-3-fresh-home-no-token": { avg: 3000, p95: 3150, runs: [] },
      },
      parseBaselineMarkdown(baselineMarkdown, "ubuntu-latest"),
    );
    expect(checks.every((c) => c.ok)).toBe(true);
  });

  test("fails when p95 exceeds five percent over baseline", () => {
    const checks = evaluateDeltas(
      {
        "scenario-1-warm-redeploy": { avg: 1000, p95: 1051, runs: [] },
        "scenario-2-cold-first-deploy": { avg: 2000, p95: 2000, runs: [] },
        "scenario-3-fresh-home-no-token": { avg: 3000, p95: 3000, runs: [] },
      },
      parseBaselineMarkdown(baselineMarkdown, "ubuntu-latest"),
    );
    expect(checks[0].ok).toBe(false);
    expect(formatDeltas(checks, "ubuntu-latest")).toContain(
      "FAIL scenario-1-warm-redeploy",
    );
  });

  test("normalizes GitHub runner OS names", () => {
    expect(normalizeBaselineOs("Linux")).toBe("ubuntu-latest");
    expect(normalizeBaselineOs("macOS")).toBe("macos-latest");
    expect(normalizeBaselineOs("Windows")).toBe("windows-latest");
  });
});
