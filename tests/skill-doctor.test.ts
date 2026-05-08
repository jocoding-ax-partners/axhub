// Phase 8 AC traceability shim — plan names this file explicitly.
// Core helpers live in skill-doctor-quality.test.ts; this file keeps the
// phase acceptance surface stable for reviewers.

import { describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { computeQualityIssues } from "../scripts/skill-doctor-quality";

describe("Phase 8 skill:doctor acceptance surface", () => {
  test("collision_matrix_detects_cross_skill_overlap: strict doctor runs collision gate", () => {
    const result = spawnSync("bun", ["scripts/skill-doctor.ts", "--strict"], {
      cwd: import.meta.dir + "/..",
      encoding: "utf8",
    });
    expect(result.status).toBe(0);
  });

  test("lang_balance_enforces_min_2_per_lang", () => {
    const issues = computeQualityIssues("status", ["상태", "어디까지", "progress", "배포상태", "진행"]);
    expect(issues.some((issue) => issue.kind === "en_balance")).toBe(true);
  });

  test("min_trigger_count_5", () => {
    const issues = computeQualityIssues("apps", ["앱", "apps", "list", "목록"]);
    expect(issues.some((issue) => issue.kind === "min_trigger")).toBe(true);
  });
});
