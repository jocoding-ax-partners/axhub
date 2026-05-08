// Phase 9 sub-task 9.3 — routing-tune.ts unit tests (mock LlmClient).

import { describe, expect, test } from "bun:test";
import {
  findFailingCases,
  runTune,
  type BaselineEntry,
  type CorpusRow,
  type FailingCase,
  type LlmClient,
} from "../scripts/routing-tune";

const SAMPLE_CORPUS: CorpusRow[] = [
  { id: "T1", utterance: "내 앱 목록 보여줘", expected_skill: "apps" },
  { id: "T2", utterance: "이거 배포해줘", expected_skill: "deploy" },
  { id: "T3", utterance: "로그 보여줘", expected_skill: "logs" },
  { id: "M1", utterance: "이 코드 어떻게 동작해?", expected_skill: null },
];

const DOCS_ONLY: BaselineEntry[] = [
  { utterance_id: "T1", fired_skill: "apps" },
  { utterance_id: "T2", fired_skill: "deploy" },
  { utterance_id: "T3", fired_skill: "deploy" },
  { utterance_id: "M1", fired_skill: null },
];

const CLAUDE_NATIVE: BaselineEntry[] = [
  { utterance_id: "T1", fired_skill: "apps" },
  { utterance_id: "T2", fired_skill: "deploy" },
  { utterance_id: "T3", fired_skill: "logs" },
  { utterance_id: "M1", fired_skill: null },
];

class MockLlmClient implements LlmClient {
  public callCount = 0;
  async suggest(failing: FailingCase) {
    this.callCount += 1;
    return {
      description_additions: [`mock-add-for-${failing.utterance_id}`],
      example_additions: [{ utterance: failing.utterance, intent: `axhub ${failing.expected_skill} intent` }],
    };
  }
}

describe("findFailingCases", () => {
  test("filters drift > 0 rows only — T3 docs-only fired_skill mismatch", () => {
    const cases = findFailingCases({
      corpus: SAMPLE_CORPUS,
      docsOnly: DOCS_ONLY,
      claudeNative: CLAUDE_NATIVE,
    });
    const t3 = cases.filter((c) => c.utterance_id === "T3");
    expect(t3.length).toBeGreaterThanOrEqual(1);
    expect(t3.some((c) => c.source === "docs-only")).toBe(true);
    expect(t3.some((c) => c.source === "drift")).toBe(true);
  });

  test("expected_skill = null rows skipped", () => {
    const cases = findFailingCases({
      corpus: SAMPLE_CORPUS,
      docsOnly: DOCS_ONLY,
      claudeNative: CLAUDE_NATIVE,
    });
    expect(cases.every((c) => c.utterance_id !== "M1")).toBe(true);
  });

  test("matching baselines produce no failing cases for that row", () => {
    const cases = findFailingCases({
      corpus: SAMPLE_CORPUS,
      docsOnly: DOCS_ONLY,
      claudeNative: CLAUDE_NATIVE,
    });
    expect(cases.every((c) => c.utterance_id !== "T1")).toBe(true);
    expect(cases.every((c) => c.utterance_id !== "T2")).toBe(true);
  });
});

describe("runTune", () => {
  test("llm_call_mock_returns_valid_json: mock LLM → suggestion JSON parse", async () => {
    const failing: FailingCase[] = [
      {
        utterance_id: "T3",
        utterance: "로그 보여줘",
        expected_skill: "logs",
        actual_skill: "deploy",
        source: "docs-only",
      },
    ];
    const llm = new MockLlmClient();
    const suggestions = await runTune({
      failingCases: failing,
      llm,
      skillsDir: "skills",
    });
    expect(suggestions.length).toBe(1);
    expect(suggestions[0]?.skill).toBe("logs");
    expect(suggestions[0]?.description_additions).toContain("mock-add-for-T3");
    expect(suggestions[0]?.example_additions[0]?.utterance).toBe("로그 보여줘");
    expect(llm.callCount).toBe(1);
  });

  test("skill_filter_works: --skill flag → only that SKILL processed", async () => {
    const failing: FailingCase[] = [
      {
        utterance_id: "T3",
        utterance: "로그 보여줘",
        expected_skill: "logs",
        actual_skill: "deploy",
        source: "docs-only",
      },
      {
        utterance_id: "T2",
        utterance: "배포해",
        expected_skill: "deploy",
        actual_skill: "logs",
        source: "drift",
      },
    ];
    const llm = new MockLlmClient();
    const suggestions = await runTune({
      failingCases: failing,
      llm,
      skillFilter: "logs",
      skillsDir: "skills",
    });
    expect(suggestions.length).toBe(1);
    expect(suggestions[0]?.skill).toBe("logs");
    expect(llm.callCount).toBe(1);
  });

  test("dry_run_outputs_suggestions_no_files_changed: runTune does not touch filesystem", async () => {
    const failing: FailingCase[] = [
      {
        utterance_id: "T3",
        utterance: "로그",
        expected_skill: "logs",
        actual_skill: "deploy",
        source: "docs-only",
      },
    ];
    const llm = new MockLlmClient();
    const before = await Bun.file("skills/logs/SKILL.md").text();
    await runTune({ failingCases: failing, llm, skillsDir: "skills" });
    const after = await Bun.file("skills/logs/SKILL.md").text();
    expect(after).toBe(before);
  });
});
