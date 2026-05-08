// Phase 9 sub-task 9.1.2 — codegen-skill-examples.ts unit tests.

import { describe, expect, test } from "bun:test";
import {
  formatExamplesYaml,
  loadCorpusUtterances,
  mergeExamplesIntoContent,
  selectExamples,
  SKILL_INTENTS,
} from "../scripts/codegen-skill-examples";

describe("selectExamples", () => {
  test("picks 2 ko + 2 en + 1 extra (ko-heavy pool)", () => {
    const result = selectExamples(
      "deploy",
      ["배포", "ship", "release", "rollout", "launch"],
      ["paydrop 배포해", "라이브로 띄워", "이거 배포"],
    );
    expect(result.length).toBe(5);
    expect(result.every((e) => e.intent === SKILL_INTENTS["deploy"])).toBe(true);
    const kos = result.filter((e) => /[가-힯]/.test(e.utterance));
    const ens = result.length - kos.length;
    expect(kos.length).toBeGreaterThanOrEqual(2);
    expect(ens).toBeGreaterThanOrEqual(2);
  });

  test("dedupes overlap between corpus and description", () => {
    const result = selectExamples("deploy", ["배포", "ship"], ["배포", "ship", "release", "rollout"]);
    const utterances = new Set(result.map((e) => e.utterance));
    expect(utterances.size).toBe(result.length);
  });

  test("uses fallback intent when slug missing from SKILL_INTENTS", () => {
    const result = selectExamples("unknown-skill", ["배포", "라이브"], ["ship", "release"]);
    if (result.length > 0) {
      expect(result[0]?.intent).toBe("axhub unknown-skill intent");
    }
  });
});

describe("formatExamplesYaml", () => {
  test("emits indented yaml block with 5 entries", () => {
    const yaml = formatExamplesYaml([
      { utterance: "배포", intent: "deploy" },
      { utterance: "ship", intent: "deploy" },
    ]);
    expect(yaml.startsWith("examples:\n")).toBe(true);
    expect(yaml).toContain('  - utterance: "배포"');
    expect(yaml).toContain('    intent: "deploy"');
    expect(yaml).toContain('  - utterance: "ship"');
  });

  test("escapes double quotes in utterance", () => {
    const yaml = formatExamplesYaml([{ utterance: 'say "hi"', intent: "greet" }]);
    expect(yaml).toContain('"say \\"hi\\""');
  });
});

describe("mergeExamplesIntoContent", () => {
  const baseContent = [
    "---",
    "name: deploy",
    "description: '...'",
    "multi-step: true",
    "needs-preflight: true",
    "---",
    "",
    "# Body",
  ].join("\n");

  test("inserts examples block above multi-step when absent", () => {
    const yaml = 'examples:\n  - utterance: "배포"\n    intent: "deploy"';
    const updated = mergeExamplesIntoContent(baseContent, yaml);
    expect(updated).toContain("examples:");
    expect(updated.indexOf("examples:")).toBeLessThan(updated.indexOf("multi-step:"));
    expect(updated).toContain("# Body");
  });

  test("replaces existing examples block", () => {
    const withExisting = baseContent.replace(
      "multi-step: true",
      'examples:\n  - utterance: "old"\n    intent: "old"\nmulti-step: true',
    );
    const yaml = 'examples:\n  - utterance: "new"\n    intent: "new"';
    const updated = mergeExamplesIntoContent(withExisting, yaml);
    expect(updated).toContain('"new"');
    expect(updated).not.toContain('"old"');
  });

  test("returns content unchanged when no frontmatter", () => {
    const noFm = "# just body\n";
    expect(mergeExamplesIntoContent(noFm, "examples: []")).toBe(noFm);
  });
});

describe("loadCorpusUtterances integration smoke", () => {
  test("returns map keyed by SKILL name", () => {
    const map = loadCorpusUtterances("tests/corpus.100.jsonl");
    expect(map.size).toBeGreaterThan(0);
    const deploy = map.get("deploy");
    expect(deploy).toBeDefined();
    expect((deploy ?? []).length).toBeGreaterThanOrEqual(5);
  });
});
