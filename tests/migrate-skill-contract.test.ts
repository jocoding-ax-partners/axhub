import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");

const read = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");

interface CorpusRow {
  id: string;
  utterance: string;
  expected_skill: string | null;
  expected_cmd_pattern: string | null;
}

interface BaselineRow {
  utterance_id?: string;
  fired_skill?: string | null;
}

const parseJsonl = (path: string) =>
  read(path)
    .trim()
    .split("\n")
    .filter(Boolean)
    .map((line) => JSON.parse(line) as CorpusRow);

const requiredUtterances = [
  "기존 앱 올려줘",
  "migrate this repo",
  "import existing app",
  "이미 만든 앱 배포해줘",
];

describe("migrate SKILL contract", () => {
  test("keeps migration behind CLI/helper boundaries instead of raw backend endpoints", () => {
    const skill = read("skills/migrate/SKILL.md");
    expect(skill).toContain("CLI boundary contract");
    expect(skill).toContain("axhub apps create --from-file axhub.yaml --yes --json");
    expect(skill).toContain('axhub apps git connect --app "$APP_ID" --repo "$OWNER_REPO" --branch "$BRANCH" --execute --json');
    expect(skill).toContain('axhub deploy create --app "$APP_ID" --branch "$BRANCH" --commit "$COMMIT_SHA" --json');
    expect(skill).toContain("backend detect endpoint 를 직접 curl 하지 않아요");
    expect(skill).not.toContain("/api/v1/apps/detect");
  });

  test("corpus.100 routes core existing-app natural language to migrate", () => {
    const rows = parseJsonl("tests/corpus.100.jsonl");
    for (const utterance of requiredUtterances) {
      const row = rows.find((candidate) => candidate.utterance === utterance);
      if (!row) {
        throw new Error(`missing migrate corpus row: ${utterance}`);
      }
      expect(row.expected_skill).toBe("migrate");
      expect(row.expected_cmd_pattern).toContain("axhub-helpers migrate-plan");
    }
  });

  test("committed routing baselines preserve migrate decisions for core utterances", () => {
    const rows = parseJsonl("tests/corpus.100.jsonl").filter((row) =>
      requiredUtterances.includes(row.utterance),
    );
    const ids = rows.map((row) => row.id);
    for (const path of [
      "tests/baseline-results.docs-only.100.json",
      "tests/baseline-results.claude-native.100.json",
    ]) {
      const baseline = JSON.parse(read(path)) as BaselineRow[];
      for (const id of ids) {
        const entry = baseline.find((row) => row.utterance_id === id);
        if (!entry) {
          throw new Error(`missing migrate baseline row: ${path}:${id}`);
        }
        expect(entry.fired_skill).toBe("migrate");
      }
    }
  });
});
