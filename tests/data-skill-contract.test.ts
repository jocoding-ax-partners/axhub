import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");

const read = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");

describe("data SKILL contract", () => {
  test("data skill exists with scaffold-required frontmatter and CLI-only workflow", () => {
    const skill = read("skills/data/SKILL.md");
    expect(skill).toContain("name: data");
    expect(skill).toContain("multi-step: true");
    expect(skill).toContain("needs-preflight: true");
    expect(skill).toContain("model: sonnet");
    expect(skill).toContain("TodoWrite({ todos:");
    expect(skill).toContain("Non-interactive AskUserQuestion guard (D1)");
    expect(skill).toContain("axhub-helpers sync");
    expect(skill).toContain("axhub-helpers snippet");
    expect(skill).toContain("axhub catalog search --json --limit 200");
    expect(skill).toContain("catalog invoke");
    expect(skill).toContain("--execute --json");
    expect(skill).toContain("catalog context 를 처음 만들까요?");
    expect(skill).toContain("Skip sync");
    expect(skill).toContain("Create context");
  });

  test("data skill documents live-read consent and no governance/path guessing rules", () => {
    const skill = read("skills/data/SKILL.md");
    expect(skill).toContain("first live read");
    expect(skill).toContain("row limit");
    expect(skill).toContain("allowed_columns");
    expect(skill).toContain("NEVER governance");
    expect(skill).toContain("NEVER path guessing");
    expect(skill).toContain("NEVER retry denied");
  });

  test("data skill hands dynamic app table work back to tables", () => {
    const skill = read("skills/data/SKILL.md");

    expect(skill).toContain("dynamic app tables are not catalog data");
    expect(skill).toContain("orders 동적 테이블 만들고 title:text 컬럼 추가해");
    expect(skill).toContain("skills/tables/SKILL.md");
    expect(skill).toContain("이 skill 에서 `axhub tables` 명령을 실행하지 않아요");
  });
});

describe("data AskUserQuestion safe defaults", () => {
  test("first catalog sync defaults to no workspace mutation in non-interactive mode", () => {
    const registry = JSON.parse(read("tests/fixtures/ask-defaults/registry.json"));
    const firstSync = registry.data["catalog context 를 처음 만들까요?"];
    expect(firstSync.safe_default).toBe("Skip sync");
    expect(firstSync.allowed_safe_defaults).toContain("Create context");
    expect(firstSync.rationale).toContain(".gitignore");
  });
});

describe("data routing corpus", () => {
  test("corpus includes data-question routing examples that do not route to apps or my-resources", () => {
    const corpus = read("tests/corpus.100.jsonl")
      .trim()
      .split("\n")
      .filter(Boolean)
      .map((line) => JSON.parse(line));
    const dataRows = corpus.filter((row) => row.expected_skill === "data");
    expect(dataRows.length).toBeGreaterThanOrEqual(3);
    for (const row of dataRows) {
      expect(row.expected_skill).not.toBe("apps");
      expect(row.expected_skill).not.toBe("my-resources");
      expect(row.expected_cmd_pattern).toContain("axhub catalog");
      expect(row.destructive).toBe(false);
    }
  });
});
