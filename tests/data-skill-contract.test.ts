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
});

describe("data routing corpus", () => {
  test("corpus includes data-question routing examples that do not route to apps or inventory", () => {
    const corpus = read("tests/corpus.100.jsonl")
      .trim()
      .split("\n")
      .filter(Boolean)
      .map((line) => JSON.parse(line));
    const dataRows = corpus.filter((row) => row.expected_skill === "data");
    expect(dataRows.length).toBeGreaterThanOrEqual(3);
    for (const row of dataRows) {
      expect(row.expected_skill).not.toBe("apps");
      expect(row.expected_skill).not.toBe("inventory");
      expect(row.expected_cmd_pattern).toContain("axhub catalog");
      expect(row.destructive).toBe(false);
    }
  });
});
