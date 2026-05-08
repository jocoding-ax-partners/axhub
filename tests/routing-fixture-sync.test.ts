import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { analyzeRoutingFixtureSync } from "../scripts/check-routing-fixture-sync";

const REPO_ROOT = join(import.meta.dir, "..");

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

  test("passes for SKILL body-only changes when routing metadata is unchanged", () => {
    const report = analyzeRoutingFixtureSync(["skills/deploy/SKILL.md"], {
      isSkillRoutingMetadataChanged: () => false,
    });
    expect(report.ok).toBe(true);
    expect(report.routingAffectingFiles).toEqual([]);
  });

  test("passes when routing metadata and baseline fixtures change together", () => {
    const report = analyzeRoutingFixtureSync([
      "skills/deploy/SKILL.md",
      "tests/baseline-results.docs-only.100.json",
    ]);
    expect(report.ok).toBe(true);
    expect(report.baselineFiles).toEqual(["tests/baseline-results.docs-only.100.json"]);
  });

  test("workflow lets the checker own git diff failures instead of piping", () => {
    const workflow = readFileSync(join(REPO_ROOT, ".github/workflows/routing-drift.yml"), "utf8");
    expect(workflow).toContain("fetch-depth: 0");
    expect(workflow).toContain(
      'bun scripts/check-routing-fixture-sync.ts --base "origin/${{ github.base_ref }}" --head HEAD',
    );
    expect(workflow).not.toMatch(
      /git diff --name-only[^\n]*\|\s*bun scripts\/check-routing-fixture-sync\.ts/,
    );
  });
});
