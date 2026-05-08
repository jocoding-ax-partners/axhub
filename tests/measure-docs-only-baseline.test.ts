// Phase 8 — measure-docs-only-baseline.ts unit tests (mock LLM + mock stdin).

import { describe, expect, test } from "bun:test";
import {
  buildLlmPrompt,
  entryFromDecision,
  runMeasurement,
  type CorpusRow,
  type LlmClient,
  type LlmRecommendation,
  type ReviewerDecision,
  type ReviewerPrompt,
  type SkillDescription,
} from "../scripts/measure-docs-only-baseline";

const SAMPLE_SKILLS: SkillDescription[] = [
  { name: "deploy", description: "이 스킬은 사용자가 현재 브랜치를 axhub 라이브로 배포하고 싶어할 때 사용해요. 다음 표현에서 활성화: \"deploy\", \"배포해\"." },
  { name: "logs", description: "이 스킬은 사용자가 axhub 배포 로그를 보고 싶어할 때 사용해요. 다음 표현에서 활성화: \"logs\", \"로그\"." },
  { name: "apps", description: "이 스킬은 사용자가 axhub 앱 목록을 보고 싶어할 때 사용해요. 다음 표현에서 활성화: \"apps\", \"앱 목록\"." },
];

const SAMPLE_CORPUS: CorpusRow[] = [
  { id: "T1", utterance: "내 앱 목록 보여줘", intent: "apps_list", expected_skill: "apps" },
  { id: "T2", utterance: "이거 배포해줘", intent: "deploy", expected_skill: "deploy" },
  { id: "M1", utterance: "이 코드 어떻게 동작해?", intent: "meta_question", expected_skill: null },
];

class MockLlmClient implements LlmClient {
  constructor(private readonly responses: Map<string, LlmRecommendation>) {}
  async recommend(utterance: string): Promise<LlmRecommendation> {
    return this.responses.get(utterance) ?? { skill: null, confidence: "low" };
  }
}

class MockReviewerPrompt implements ReviewerPrompt {
  constructor(private readonly decisions: Map<string, ReviewerDecision>) {}
  async ask(row: CorpusRow): Promise<ReviewerDecision> {
    return this.decisions.get(row.id) ?? { kind: "skip" };
  }
}

describe("buildLlmPrompt", () => {
  test("includes utterance + skills + JSON format hint", () => {
    const prompt = buildLlmPrompt("배포해", SAMPLE_SKILLS);
    expect(prompt).toContain('"배포해"');
    expect(prompt).toContain("- deploy:");
    expect(prompt).toContain("- logs:");
    expect(prompt).toContain('"skill":');
    expect(prompt).toContain('"confidence":');
  });
});

describe("entryFromDecision", () => {
  const recommendation: LlmRecommendation = { skill: "deploy", confidence: "high" };
  const measuredAt = "2026-05-08T12:00:00Z";

  test("accept → fired_skill = recommendation.skill", () => {
    const entry = entryFromDecision(SAMPLE_CORPUS[1]!, recommendation, { kind: "accept" }, measuredAt);
    expect(entry?.fired_skill).toBe("deploy");
    expect(entry?.notes).toContain("accept (claude high)");
  });

  test("override → fired_skill = overrideSkill", () => {
    const entry = entryFromDecision(SAMPLE_CORPUS[1]!, recommendation, { kind: "override", overrideSkill: "logs" }, measuredAt);
    expect(entry?.fired_skill).toBe("logs");
    expect(entry?.notes).toContain("override");
  });

  test("null → fired_skill = null", () => {
    const entry = entryFromDecision(SAMPLE_CORPUS[2]!, recommendation, { kind: "null" }, measuredAt);
    expect(entry?.fired_skill).toBeNull();
    expect(entry?.notes).toContain("null");
  });

  test("skip → returns null entry", () => {
    const entry = entryFromDecision(SAMPLE_CORPUS[1]!, recommendation, { kind: "skip" }, measuredAt);
    expect(entry).toBeNull();
  });
});

describe("runMeasurement", () => {
  test("3 row sample with mocked LLM + reviewer produces normalized JSON", async () => {
    const llm = new MockLlmClient(
      new Map<string, LlmRecommendation>([
        ["내 앱 목록 보여줘", { skill: "apps", confidence: "high" }],
        ["이거 배포해줘", { skill: "deploy", confidence: "high" }],
        ["이 코드 어떻게 동작해?", { skill: "deploy", confidence: "low" }],
      ]),
    );
    const prompt = new MockReviewerPrompt(
      new Map<string, ReviewerDecision>([
        ["T1", { kind: "accept" }],
        ["T2", { kind: "accept" }],
        ["M1", { kind: "null" }],
      ]),
    );
    const { metadata, entries } = await runMeasurement({
      corpus: SAMPLE_CORPUS,
      skills: SAMPLE_SKILLS,
      llm,
      prompt,
      measuredAt: "2026-05-08T12:00:00Z",
      reviewer: "test-runner",
    });
    expect(entries.length).toBe(3);
    expect(entries[0]?.utterance_id).toBe("T1");
    expect(entries[0]?.fired_skill).toBe("apps");
    expect(entries[1]?.fired_skill).toBe("deploy");
    expect(entries[2]?.fired_skill).toBeNull();
    expect(metadata._metadata.rows_measured).toBe(3);
    expect(metadata._metadata.rows_skipped).toBe(0);
    expect(metadata._metadata.decisions).toEqual({ accept: 2, override: 0, null: 1 });
    expect(metadata._metadata.reviewer).toBe("test-runner");
  });

  test("skip decision drops row + increments rows_skipped", async () => {
    const llm = new MockLlmClient(new Map([["x", { skill: null, confidence: "low" } as LlmRecommendation]]));
    const prompt = new MockReviewerPrompt(new Map<string, ReviewerDecision>([["X", { kind: "skip" }]]));
    const corpus: CorpusRow[] = [{ id: "X", utterance: "x", intent: "x", expected_skill: null }];
    const { metadata, entries } = await runMeasurement({ corpus, skills: SAMPLE_SKILLS, llm, prompt });
    expect(entries.length).toBe(0);
    expect(metadata._metadata.rows_skipped).toBe(1);
  });

  test("override decision increments override counter + records overrideSkill", async () => {
    const llm = new MockLlmClient(new Map([["배포해", { skill: "logs", confidence: "low" } as LlmRecommendation]]));
    const prompt = new MockReviewerPrompt(
      new Map<string, ReviewerDecision>([["O1", { kind: "override", overrideSkill: "deploy" }]]),
    );
    const corpus: CorpusRow[] = [{ id: "O1", utterance: "배포해", intent: "deploy", expected_skill: "deploy" }];
    const { metadata, entries } = await runMeasurement({ corpus, skills: SAMPLE_SKILLS, llm, prompt });
    expect(entries.length).toBe(1);
    expect(entries[0]?.fired_skill).toBe("deploy");
    expect(metadata._metadata.decisions.override).toBe(1);
  });
});
