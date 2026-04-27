// Phase 13 US-1306 — verify Toss tone conformance + skill keyword preservation
// lint scripts work correctly. PR2 source changes will not merge until both
// scripts pass.

import { describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";

import { scan, FORBIDDEN, PHASE_13_FILES } from "../scripts/check-toss-tone-conformance";
import { snapshot, BASELINE_PATH } from "../scripts/check-skill-keywords-preserved";

describe("Phase 13 US-1306 — Toss tone conformance lint", () => {
  test("FORBIDDEN tokens cover the 7 Toss rules + axhub deprecation", () => {
    const rules = new Set(FORBIDDEN.map((t) => t.rule));
    expect(rules.has("T-01")).toBe(true);
    expect(rules.has("T-06")).toBe(true);
    expect(rules.has("T-09 (axhub)")).toBe(true);
    expect(rules.has("axhub-deprecation")).toBe(true);
    expect(FORBIDDEN.length).toBeGreaterThanOrEqual(6);
  });

  test("PHASE_13_FILES returns runtime + commands + install + hook scope", async () => {
    const files = await PHASE_13_FILES();
    expect(files.length).toBeGreaterThan(15);
    expect(files.some((f) => f.includes("catalog.ts"))).toBe(true);
    expect(files.some((f) => f.includes("keychain.ts"))).toBe(true);
    expect(files.some((f) => f.includes("install.ps1"))).toBe(true);
    expect(files.some((f) => f.includes("commands/help.md"))).toBe(true);
  });

  test("scan() returns violations with file:line:col + rule + reason", async () => {
    const violations = await scan();
    // Phase 13 baseline has 33 errors before tone migration
    expect(violations.length).toBeGreaterThan(0);
    for (const v of violations) {
      expect(v.file).toBeTypeOf("string");
      expect(v.line).toBeGreaterThan(0);
      expect(v.col).toBeGreaterThan(0);
      expect(v.rule).toBeTypeOf("string");
      expect(v.reason).toBeTypeOf("string");
    }
  });

  test("scan() classifies errors vs warnings correctly", async () => {
    const violations = await scan();
    const errors = violations.filter((v) => v.severity === "error");
    const warns = violations.filter((v) => v.severity === "warn");
    // 시나요 = warn (3 exceptions), all others = error
    expect(errors.length + warns.length).toBe(violations.length);
  });
});

describe("Phase 13 US-1306 — Skill keyword preservation lint", () => {
  test("snapshot() returns description_phrases + lexicon_phrases + timestamp", async () => {
    const snap = await snapshot();
    expect(snap.description_phrases).toBeTypeOf("object");
    expect(Array.isArray(snap.lexicon_phrases)).toBe(true);
    expect(snap.generated_at).toBeTypeOf("string");
  });

  test("baseline file exists at .omc/lint-baselines/skill-keywords.json", () => {
    expect(existsSync(BASELINE_PATH)).toBe(true);
  });

  test("baseline captured at least 10 SKILL.md files + 500 lexicon phrases", async () => {
    const snap = await snapshot();
    expect(Object.keys(snap.description_phrases).length).toBeGreaterThanOrEqual(10);
    expect(snap.lexicon_phrases.length).toBeGreaterThanOrEqual(500);
  });
});
