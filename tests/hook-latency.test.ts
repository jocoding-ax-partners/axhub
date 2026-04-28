// M4 hook latency benchmark wiring — keep timing assertions in scripts/benchmark-hooks.ts,
// not in unit tests, to avoid flaky CI failures on shared runners.

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import packageJson from "../package.json" with { type: "json" };

const REPO_ROOT = join(import.meta.dir, "..");
const BENCH = join(REPO_ROOT, "scripts/benchmark-hooks.ts");
const PLAN = join(REPO_ROOT, "PLAN.md");
const INDEX_TS = join(REPO_ROOT, "src/axhub-helpers/index.ts");

describe("M4 hook no-op latency benchmark", () => {
  test("package.json exposes bench:hooks", () => {
    expect(packageJson.scripts["bench:hooks"]).toBe("bun scripts/benchmark-hooks.ts");
  });

  test("benchmark locks the 50ms p95 hot-path gate and both no-op hooks", () => {
    const content = readFileSync(BENCH, "utf8");
    expect(content).toContain('AXHUB_HOOK_LATENCY_P95_MS"] ?? "50"');
    expect(content).toContain('subcommand: "preauth-check"');
    expect(content).toContain('subcommand: "classify-exit"');
    expect(content).toContain('tool_input: { command: "echo hello" }');
    expect(content).toContain("permissionDecision");
    expect(content).toContain("expected PostToolUse no-op {}");
  });

  test("PLAN and helper comments no longer promise an impossible 5ms compiled-binary gate", () => {
    const plan = readFileSync(PLAN, "utf8");
    const index = readFileSync(INDEX_TS, "utf8");
    expect(plan).toContain("50ms p95");
    expect(plan).toContain("bun run bench:hooks");
    expect(index).toContain("50ms hot path goal");
    expect(plan).not.toContain("hook latency benchmark < 5ms");
    expect(index).not.toContain("5ms gate");
  });
});
