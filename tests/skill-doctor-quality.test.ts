// Phase 8 sub-task 8.2 — skill-doctor quality lint helpers.
// computeQualityIssues + isKorean unit tests with mock phrase fixtures.

import { describe, expect, test } from "bun:test";
import {
  computeExamplesIssues,
  computeQualityIssues,
  isKorean,
  MIN_EXAMPLES,
  MIN_PER_LANG,
  MIN_TRIGGER_COUNT,
  parseExamples,
} from "../scripts/skill-doctor-quality";

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

describe("parseExamples", () => {
  test("parses 2 example pairs from frontmatter", () => {
    const fm = [
      "name: deploy",
      "examples:",
      '  - utterance: "배포해"',
      '    intent: "deploy current branch"',
      '  - utterance: "ship it"',
      '    intent: "deploy current branch"',
      "multi-step: true",
    ].join("\n");
    const examples = parseExamples(fm);
    expect(examples.length).toBe(2);
    expect(examples[0]?.utterance).toBe("배포해");
    expect(examples[0]?.intent).toBe("deploy current branch");
    expect(examples[1]?.utterance).toBe("ship it");
  });

  test("returns empty when examples field absent", () => {
    const fm = "name: foo\ndescription: 'bar'\n";
    expect(parseExamples(fm)).toEqual([]);
  });

  test("stops at next top-level field", () => {
    const fm = [
      "examples:",
      '  - utterance: "a"',
      '    intent: "intent a"',
      "multi-step: true",
      '  - utterance: "should-not-appear"',
      '    intent: "garbage"',
    ].join("\n");
    expect(parseExamples(fm).length).toBe(1);
  });
});

describe("computeExamplesIssues", () => {
  const validFm = [
    "examples:",
    '  - utterance: "배포해"',
    '    intent: "deploy current branch to axhub live"',
    '  - utterance: "라이브로 띄워"',
    '    intent: "deploy current branch to axhub live"',
    '  - utterance: "ship it"',
    '    intent: "deploy current branch to axhub live"',
    '  - utterance: "release now"',
    '    intent: "deploy current branch to axhub live"',
    '  - utterance: "rollout"',
    '    intent: "deploy current branch to axhub live"',
  ].join("\n");

  test("missing examples field → examples_missing", () => {
    const issues = computeExamplesIssues("foo", "name: foo\n");
    expect(issues.some((i) => i.kind === "examples_missing")).toBe(true);
  });

  test("< MIN_EXAMPLES → examples_min", () => {
    const fm = [
      "examples:",
      '  - utterance: "한"',
      '    intent: "intent"',
    ].join("\n");
    const issues = computeExamplesIssues("foo", fm);
    expect(issues.some((i) => i.kind === "examples_min")).toBe(true);
    expect(MIN_EXAMPLES).toBe(5);
  });

  test("ko < 2 → examples_lang_ko, en < 2 → examples_lang_en", () => {
    const koHeavy = [
      "examples:",
      '  - utterance: "배포"',
      '    intent: "deploy"',
      '  - utterance: "라이브"',
      '    intent: "deploy"',
      '  - utterance: "올려"',
      '    intent: "deploy"',
      '  - utterance: "쏘자"',
      '    intent: "deploy"',
      '  - utterance: "ship"',
      '    intent: "deploy"',
    ].join("\n");
    const issues = computeExamplesIssues("foo", koHeavy);
    expect(issues.some((i) => i.kind === "examples_lang_en")).toBe(true);
  });

  test("intent contains 한국어 → intent_lang", () => {
    const fm = [
      "examples:",
      '  - utterance: "배포"',
      '    intent: "배포 의도"',
      '  - utterance: "라이브"',
      '    intent: "deploy"',
      '  - utterance: "ship"',
      '    intent: "deploy"',
      '  - utterance: "release"',
      '    intent: "deploy"',
      '  - utterance: "rollout"',
      '    intent: "deploy"',
    ].join("\n");
    const issues = computeExamplesIssues("foo", fm);
    expect(issues.some((i) => i.kind === "intent_lang")).toBe(true);
  });

  test("balanced + 5 examples + en intent → 0 issues", () => {
    expect(computeExamplesIssues("foo", validFm)).toEqual([]);
  });
});
