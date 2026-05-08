import { describe, expect, test } from "bun:test";
import { analyzeRoutingFixtureSync } from "../scripts/check-routing-fixture-sync";

describe("routing fixture sync guard", () => {
  test("passes when routing files are untouched", () => {
    const report = analyzeRoutingFixtureSync(["docs/routing.md", "README.md"]);
    expect(report.ok).toBe(true);
    expect(report.routingAffectingFiles).toEqual([]);
  });

  test("fails when SKILL routing metadata changes without baseline fixtures", () => {
    const report = analyzeRoutingFixtureSync(["skills/deploy/SKILL.md"]);
    expect(report.ok).toBe(false);
    expect(report.routingAffectingFiles).toEqual(["skills/deploy/SKILL.md"]);
  });

  test("passes when routing metadata and baseline fixtures change together", () => {
    const report = analyzeRoutingFixtureSync([
      "skills/deploy/SKILL.md",
      "tests/baseline-results.docs-only.100.json",
    ]);
    expect(report.ok).toBe(true);
    expect(report.baselineFiles).toEqual(["tests/baseline-results.docs-only.100.json"]);
  });
});
