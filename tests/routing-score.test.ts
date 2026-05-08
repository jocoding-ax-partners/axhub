// Phase 0 — Sub-task 0.1: routing-score.ts unit tests.
import { describe, expect, test } from "bun:test";
import { score, drift } from "./routing-score";
import type { CorpusRow, ResultRow } from "./routing-score";

const corpus: CorpusRow[] = [
  { id: "A1", utterance: "배포해줘", intent: "deploy", expected_skill: "deploy", expected_cmd_pattern: "axhub deploy", destructive: false, lang: "ko" },
  { id: "A2", utterance: "로그 봐", intent: "logs", expected_skill: "logs", expected_cmd_pattern: "axhub logs", destructive: false, lang: "ko" },
  { id: "N1", utterance: "git push", intent: "git_push", expected_skill: null, expected_cmd_pattern: null, destructive: false, lang: "en" },
  { id: "M1", utterance: "왜 배포가 keyword 매칭이지?", intent: "meta_question", expected_skill: null, expected_cmd_pattern: null, destructive: false, lang: "ko" },
];

describe("score()", () => {
  test("100% match — all positive + all null rejected", () => {
    const results: ResultRow[] = [
      { utterance_id: "A1", fired_skill: "deploy" },
      { utterance_id: "A2", fired_skill: "logs" },
      { utterance_id: "N1", fired_skill: null },
      { utterance_id: "M1", fired_skill: null },
    ];
    const s = score(corpus, results);
    expect(s.total).toBe(4);
    expect(s.matched).toBe(4);
    expect(s.overall_accuracy).toBe(1);
    expect(s.null_rejection_rate).toBe(1);
    expect(s.missing_results).toBe(0);
    expect(s.extra_results).toBe(0);
    expect(s.per_skill.deploy?.precision).toBe(1);
    expect(s.per_skill.deploy?.recall).toBe(1);
  });

  test("null missed (false-positive routing of meta_question)", () => {
    const results: ResultRow[] = [
      { utterance_id: "A1", fired_skill: "deploy" },
      { utterance_id: "A2", fired_skill: "logs" },
      { utterance_id: "N1", fired_skill: null },
      { utterance_id: "M1", fired_skill: "deploy" },
    ];
    const s = score(corpus, results);
    expect(s.matched).toBe(3);
    expect(s.null_missed).toBe(1);
    expect(s.null_rejected).toBe(1);
    expect(s.null_rejection_rate).toBe(0.5);
    expect(s.overall_accuracy).toBe(0.75);
    expect(s.per_skill.deploy?.fp).toBe(1);
  });

  test("positive wrong (deploy → logs)", () => {
    const results: ResultRow[] = [
      { utterance_id: "A1", fired_skill: "logs" },
      { utterance_id: "A2", fired_skill: "logs" },
      { utterance_id: "N1", fired_skill: null },
      { utterance_id: "M1", fired_skill: null },
    ];
    const s = score(corpus, results);
    expect(s.matched).toBe(3);
    expect(s.positive_wrong).toBe(1);
    expect(s.per_skill.deploy?.fn).toBe(1);
    expect(s.per_skill.logs?.fp).toBe(1);
    expect(s.per_skill.logs?.tp).toBe(1);
  });

  test("positive missed (deploy → null)", () => {
    const results: ResultRow[] = [
      { utterance_id: "A1", fired_skill: null },
      { utterance_id: "A2", fired_skill: "logs" },
      { utterance_id: "N1", fired_skill: null },
      { utterance_id: "M1", fired_skill: null },
    ];
    const s = score(corpus, results);
    expect(s.matched).toBe(3);
    expect(s.positive_missed).toBe(1);
    expect(s.per_skill.deploy?.fn).toBe(1);
  });

  test("missing result rows count against corpus total", () => {
    const s = score(corpus, [
      { utterance_id: "A1", fired_skill: "deploy" },
      { utterance_id: "N1", fired_skill: null },
    ]);
    expect(s.total).toBe(4);
    expect(s.matched).toBe(2);
    expect(s.missing_results).toBe(2);
    expect(s.positive_missed).toBe(1);
    expect(s.overall_accuracy).toBe(0.5);
    expect(s.per_skill.logs?.fn).toBe(1);
  });

  test("empty results still uses corpus total and fails accuracy", () => {
    const s = score(corpus, []);
    expect(s.total).toBe(4);
    expect(s.missing_results).toBe(4);
    expect(s.positive_missed).toBe(2);
    expect(s.overall_accuracy).toBe(0);
  });

  test("unknown result rows are reported as extra fixture data", () => {
    const s = score(corpus, [
      { utterance_id: "A1", fired_skill: "deploy" },
      { utterance_id: "UNKNOWN", fired_skill: "deploy" },
    ]);
    expect(s.extra_results).toBe(1);
    expect(s.missing_results).toBe(3);
    expect(s.overall_accuracy).toBe(0.25);
  });
});

describe("drift()", () => {
  test("identical baselines — 0% drift", () => {
    const a: ResultRow[] = [
      { utterance_id: "A1", fired_skill: "deploy" },
      { utterance_id: "A2", fired_skill: "logs" },
    ];
    const b: ResultRow[] = [
      { utterance_id: "A1", fired_skill: "deploy" },
      { utterance_id: "A2", fired_skill: "logs" },
    ];
    const d = drift(a, b);
    expect(d.total_rows).toBe(2);
    expect(d.mismatching_rows).toBe(0);
    expect(d.mismatch_rate).toBe(0);
  });

  test("50% drift", () => {
    const a: ResultRow[] = [
      { utterance_id: "A1", fired_skill: "deploy" },
      { utterance_id: "A2", fired_skill: "logs" },
    ];
    const b: ResultRow[] = [
      { utterance_id: "A1", fired_skill: "deploy" },
      { utterance_id: "A2", fired_skill: "status" },
    ];
    const d = drift(a, b);
    expect(d.mismatching_rows).toBe(1);
    expect(d.mismatch_rate).toBe(0.5);
    expect(d.mismatches[0]?.id).toBe("A2");
    expect(d.mismatches[0]?.baseline).toBe("logs");
    expect(d.mismatches[0]?.against).toBe("status");
  });

  test("non-overlapping ids — 0 rows compared", () => {
    const a: ResultRow[] = [{ utterance_id: "A1", fired_skill: "deploy" }];
    const b: ResultRow[] = [{ utterance_id: "B1", fired_skill: "logs" }];
    const d = drift(a, b);
    expect(d.total_rows).toBe(0);
    expect(d.mismatch_rate).toBe(0);
  });
});
