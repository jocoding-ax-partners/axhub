// Phase 6 — migration-gate-check.ts unit tests (mock-based).
//
// We verify the aggregator helpers (formatMarkdown / shouldExitWithError /
// parseLatencyP95 / runGate manual short-circuit) without invoking the real
// 6 gates — those are covered by `bun run gate:check` in the PR pipeline.

import { describe, expect, test } from "bun:test";
import {
  formatMarkdown,
  parseLatencyP95,
  runGate,
  shouldExitWithError,
  type GateResult,
} from "../scripts/migration-gate-check";

const passing: GateResult = { name: "A", passed: true, evidence: "exit 0", duration_ms: 1 };
const failing: GateResult = { name: "B", passed: false, evidence: "exit 1", duration_ms: 2 };
const manual: GateResult = { name: "C", passed: true, evidence: "manual", duration_ms: 0, manual: true };

describe("formatMarkdown", () => {
  test("emits header + table rows for each gate", () => {
    const md = formatMarkdown([passing, failing, manual]);
    expect(md).toContain("## Migration Gate Evidence (Approach E)");
    expect(md).toContain("| Status | Gate | Evidence | Duration |");
    expect(md).toMatch(/\|\s*✅\s*\|\s*A\s*\|/);
    expect(md).toMatch(/\|\s*❌\s*\|\s*B\s*\|/);
    expect(md).toMatch(/\|\s*⚠️\s*\|\s*C\s*\|/);
  });

  test("escapes pipes in evidence", () => {
    const g: GateResult = { name: "X", passed: true, evidence: "stdout | tail -1", duration_ms: 0 };
    expect(formatMarkdown([g])).toContain("stdout \\| tail -1");
  });
});

describe("shouldExitWithError", () => {
  test("returns false when all gates pass", () => {
    expect(shouldExitWithError([passing, manual])).toBe(false);
  });

  test("returns true when a non-manual gate fails", () => {
    expect(shouldExitWithError([passing, failing])).toBe(true);
  });

  test("manual failure is ignored", () => {
    const manualFail: GateResult = { ...manual, passed: false };
    expect(shouldExitWithError([passing, manualFail])).toBe(false);
  });
});

describe("parseLatencyP95", () => {
  test("extracts p95 ms from benchmark text", () => {
    expect(parseLatencyP95("p95: 12.5 ms")).toBeCloseTo(12.5, 5);
    expect(parseLatencyP95("p95=42ms")).toBe(42);
  });

  test("ignores p95-threshold banner and returns worst real row", () => {
    const text = [
      "[hook-latency] samples=40 warmup=5 p95-threshold=50ms",
      "UserPromptSubmit p50=5ms p95=12ms",
      "PreToolUse p50=8ms p95=18ms",
    ].join("\n");
    expect(parseLatencyP95(text)).toBe(18);
  });

  test("returns null when missing", () => {
    expect(parseLatencyP95("no latency reported")).toBeNull();
  });
});

describe("runGate manual short-circuit", () => {
  test("manual gate does not spawn subprocess", () => {
    const r = runGate("Canary", ["manual: human-only"], { manual: true });
    expect(r.passed).toBe(true);
    expect(r.manual).toBe(true);
    expect(r.duration_ms).toBe(0);
    expect(r.evidence).toContain("manual:");
  });
});
