// Phase 8 sub-task 8.2 — skill-doctor quality lint helpers.
// computeQualityIssues + isKorean unit tests with mock phrase fixtures.

import { describe, expect, test } from "bun:test";
import { computeQualityIssues, isKorean, MIN_PER_LANG, MIN_TRIGGER_COUNT } from "../scripts/skill-doctor-quality";

describe("isKorean", () => {
  test("Korean characters", () => {
    expect(isKorean("배포")).toBe(true);
    expect(isKorean("로그")).toBe(true);
  });

  test("non-Korean", () => {
    expect(isKorean("deploy")).toBe(false);
    expect(isKorean("ship")).toBe(false);
  });
});

describe("computeQualityIssues", () => {
  test("min_trigger_count_5: < 5 phrases → min_trigger issue", () => {
    const phrases = ["a", "b", "c", "한", "글"];
    expect(phrases.length).toBe(MIN_TRIGGER_COUNT);
    const ok = computeQualityIssues("ok-skill", phrases);
    expect(ok.some((i) => i.kind === "min_trigger")).toBe(false);

    const tooFew = computeQualityIssues("tiny", ["a", "한", "b", "글"]);
    const minIssue = tooFew.find((i) => i.kind === "min_trigger");
    expect(minIssue).toBeDefined();
    expect(minIssue?.detail).toContain("4 <");
  });

  test("lang_balance_enforces_min_2_per_lang: ko < 2 → ko_balance issue", () => {
    const phrases = ["deploy", "ship", "release", "rollout", "한"];
    const issues = computeQualityIssues("en-heavy", phrases);
    const koIssue = issues.find((i) => i.kind === "ko_balance");
    expect(koIssue).toBeDefined();
    expect(koIssue?.detail).toContain(`ko=1 < ${MIN_PER_LANG}`);
  });

  test("lang_balance_enforces_min_2_per_lang: en < 2 → en_balance issue", () => {
    const phrases = ["배포", "로그", "앱", "상태", "deploy"];
    const issues = computeQualityIssues("ko-heavy", phrases);
    const enIssue = issues.find((i) => i.kind === "en_balance");
    expect(enIssue).toBeDefined();
    expect(enIssue?.detail).toContain(`en=1 < ${MIN_PER_LANG}`);
  });

  test("balanced phrases ≥5 + ko≥2 + en≥2 → no quality issue", () => {
    const phrases = ["배포", "로그", "deploy", "ship", "release"];
    const issues = computeQualityIssues("balanced", phrases);
    expect(issues.length).toBe(0);
  });

  test("multiple violations stacked", () => {
    const phrases = ["a", "b"];
    const issues = computeQualityIssues("bad", phrases);
    expect(issues.length).toBe(2);
    expect(issues.map((i) => i.kind).sort()).toEqual(["ko_balance", "min_trigger"]);
  });
});
