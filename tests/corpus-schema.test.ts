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

interface CorpusRow {
  id: string;
  utterance: string;
  intent: string;
  expected_skill: string | null;
  expected_cmd_pattern: string | null;
  destructive: boolean;
  lang: string;
}

const loadCorpus = (relPath: string): CorpusRow[] => {
  const lines = readFileSync(join(REPO_ROOT, relPath), "utf8")
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l.length > 0 && !l.startsWith("//"));
  return lines.map((l) => JSON.parse(l) as CorpusRow);
};

const corpus = loadCorpus("tests/corpus.100.jsonl");

describe("corpus.100.jsonl schema validation (US-203)", () => {
  test("at least 100 rows (100 base + Phase 5 meta_question expansion)", () => {
    expect(corpus.length).toBeGreaterThanOrEqual(100);
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
    expect(ids.size).toBe(corpus.length);
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

// Phase 5 — meta_question intent invariants across all 3 tiers.
// meta_question = "이 코드/구현/라우팅 어떻게 동작해?" 류 발화. axhub 도구 호출 의도 X →
// expected_skill = null + expected_cmd_pattern = null + destructive = false.

const tier20 = loadCorpus("tests/corpus.20.jsonl");
const tier100 = corpus;
const tierFull = loadCorpus("tests/corpus.jsonl");

describe("meta_question intent (Phase 5 — Approach E corpus expansion)", () => {
  test("meta_question rows have null expected_skill + null expected_cmd_pattern + destructive=false", () => {
    for (const tier of [tier20, tier100, tierFull]) {
      const meta = tier.filter((r) => r.intent === "meta_question");
      expect(meta.length).toBeGreaterThan(0);
      for (const row of meta) {
        expect(row.expected_skill).toBeNull();
        expect(row.expected_cmd_pattern).toBeNull();
        expect(row.destructive).toBe(false);
      }
    }
  });

  test("meta_question count per tier (20≥3, 100≥10, full≥15)", () => {
    expect(tier20.filter((r) => r.intent === "meta_question").length).toBeGreaterThanOrEqual(3);
    expect(tier100.filter((r) => r.intent === "meta_question").length).toBeGreaterThanOrEqual(10);
    expect(tierFull.filter((r) => r.intent === "meta_question").length).toBeGreaterThanOrEqual(15);
  });

  test("subset relationship: tier20 ids ⊆ tier100 ids ⊆ tierFull ids (meta only)", () => {
    const metaIds20 = new Set(tier20.filter((r) => r.intent === "meta_question").map((r) => r.id));
    const metaIds100 = new Set(tier100.filter((r) => r.intent === "meta_question").map((r) => r.id));
    const metaIdsFull = new Set(tierFull.filter((r) => r.intent === "meta_question").map((r) => r.id));
    for (const id of metaIds20) {
      expect(metaIds100.has(id)).toBe(true);
    }
    for (const id of metaIds100) {
      expect(metaIdsFull.has(id)).toBe(true);
    }
  });
});
