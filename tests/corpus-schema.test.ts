// Phase 3 US-203: corpus.100.jsonl schema validation + expected_cmd_pattern coverage.
//
// Invariants:
//   - All 100 rows have id, utterance, intent, expected_skill, expected_cmd_pattern,
//     destructive, lang fields present.
//   - Read-only rows (destructive=false, expected_skill !== null) have non-null
//     expected_cmd_pattern (architect M2.5 §3 — scoring reproducibility).
//   - Negative rows (destructive=false, expected_skill === null) have explicit
//     expected_cmd_pattern: null (signals "no axhub command should fire").
//   - All 23 destructive rows MUST have expected_cmd_pattern (cannot be null —
//     if no command should fire, it can't be destructive).

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const CORPUS_PATH = join(REPO_ROOT, "tests/corpus.100.jsonl");

interface CorpusRow {
  id: string;
  utterance: string;
  intent: string;
  expected_skill: string | null;
  expected_cmd_pattern: string | null;
  destructive: boolean;
  lang: string;
}

const loadCorpus = (): CorpusRow[] => {
  const lines = readFileSync(CORPUS_PATH, "utf8")
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l.length > 0 && !l.startsWith("//"));
  return lines.map((l) => JSON.parse(l) as CorpusRow);
};

const corpus = loadCorpus();

describe("corpus.100.jsonl schema validation (US-203)", () => {
  test("exactly 100 rows", () => {
    expect(corpus.length).toBe(100);
  });

  test("each row has all 7 required fields", () => {
    for (const row of corpus) {
      expect(row.id).toBeTypeOf("string");
      expect(row.utterance).toBeTypeOf("string");
      expect(row.intent).toBeTypeOf("string");
      expect("expected_skill" in row).toBe(true);
      expect("expected_cmd_pattern" in row).toBe(true);
      expect(row.destructive).toBeTypeOf("boolean");
      expect(row.lang).toBeTypeOf("string");
    }
  });

  test("each row id is unique", () => {
    const ids = new Set<string>();
    for (const row of corpus) {
      expect(ids.has(row.id)).toBe(false);
      ids.add(row.id);
    }
    expect(ids.size).toBe(100);
  });

  test("lang is one of: ko, en, mixed, slash", () => {
    const valid = new Set(["ko", "en", "mixed", "slash"]);
    for (const row of corpus) {
      expect(valid.has(row.lang)).toBe(true);
    }
  });
});

describe("expected_cmd_pattern coverage (US-203 + architect M2.5 §3)", () => {
  test("all destructive rows MUST have non-null expected_cmd_pattern", () => {
    const destructiveRows = corpus.filter((r) => r.destructive);
    expect(destructiveRows.length).toBeGreaterThan(0);
    for (const row of destructiveRows) {
      if (row.expected_cmd_pattern === null) {
        throw new Error(`destructive row ${row.id} has null expected_cmd_pattern`);
      }
      expect(row.expected_cmd_pattern).not.toBeNull();
    }
  });

  test("read-only rows with non-null expected_skill MUST have non-null expected_cmd_pattern", () => {
    const readOnlyWithSkill = corpus.filter((r) => !r.destructive && r.expected_skill !== null);
    expect(readOnlyWithSkill.length).toBeGreaterThan(0);
    for (const row of readOnlyWithSkill) {
      if (row.expected_cmd_pattern === null) {
        throw new Error(`read-only skill-routed row ${row.id} (skill=${row.expected_skill}) has null expected_cmd_pattern`);
      }
      expect(row.expected_cmd_pattern).not.toBeNull();
    }
  });

  test("pure-negative rows (destructive=false AND expected_skill=null) MUST have null expected_cmd_pattern", () => {
    // Destructive bypass attempts (destructive=true, expected_skill=null) are
    // separate — they have a pattern (the destructive cmd the parser must
    // detect) but no skill should fire (gate blocks at hook level).
    const pureNegative = corpus.filter((r) => !r.destructive && r.expected_skill === null);
    expect(pureNegative.length).toBeGreaterThan(0);
    for (const row of pureNegative) {
      expect(row.expected_cmd_pattern).toBeNull();
    }
  });

  test("expected_cmd_pattern values compile as valid regex", () => {
    for (const row of corpus) {
      if (row.expected_cmd_pattern === null) continue;
      expect(() => new RegExp(row.expected_cmd_pattern!)).not.toThrow();
    }
  });

  test("all expected_cmd_pattern values reference 'axhub' command (when non-null)", () => {
    for (const row of corpus) {
      if (row.expected_cmd_pattern === null) continue;
      expect(row.expected_cmd_pattern).toContain("axhub");
    }
  });
});

describe("expected_cmd_pattern distribution (curation health)", () => {
  test("destructive count is reasonable (10-50 of 100, current curation has many adversarial bypass attempts)", () => {
    const count = corpus.filter((r) => r.destructive).length;
    expect(count).toBeGreaterThanOrEqual(10);
    expect(count).toBeLessThanOrEqual(50);
  });

  test("negative-case count is reasonable (10-30 of 100)", () => {
    const count = corpus.filter((r) => r.expected_skill === null).length;
    expect(count).toBeGreaterThanOrEqual(10);
    expect(count).toBeLessThanOrEqual(30);
  });

  test("expected_skill set covers core skills", () => {
    const skills = new Set(corpus.map((r) => r.expected_skill).filter((s): s is string => s !== null));
    // At minimum we expect these skills represented in the 100-row subset.
    const required = ["apps", "apis", "deploy", "status", "logs", "auth", "update", "doctor"];
    for (const s of required) {
      expect(skills.has(s)).toBe(true);
    }
  });
});
