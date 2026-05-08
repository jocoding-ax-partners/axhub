// Phase 1 sub-task 1.4: codegen-skill-keywords-from-rust.ts unit + idempotency tests.
import { describe, expect, test } from "bun:test";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import {
  parseMainRs,
  aggregatePerSkill,
  computeDiff,
  applyMerge,
  formatPhraseList,
  readSkillDescription,
} from "../scripts/codegen-skill-keywords-from-rust";

const REPO_ROOT = join(import.meta.dir, "..");

describe("parseMainRs (Phase 2 — Approach E: detect_prompt_route 폐기 후)", () => {
  // Phase 2 후 main.rs 에 detect_prompt_route 부재 → parseMainRs() returns [].
  // Phase 1 시점 (PR #40) 에서는 19 blocks 였음. one-time migration 완료 후 의미 없음.
  test("returns empty array when detect_prompt_route is removed (Phase 2)", () => {
    const blocks = parseMainRs();
    expect(blocks.length).toBe(0);
  });

  test("upgrade description remains scoped after runtime codegen source removal", () => {
    const region = readSkillDescription(join(REPO_ROOT, "skills/upgrade/SKILL.md"));
    expect(region).not.toBeNull();
    const phrases = region?.existingPhrases ?? [];
    for (const unsafe of ["plugin", "플러그인", "update", "upgrade", "version", "업데이트", "업그레이드", "버전", "새 버전", "호환"]) {
      expect(phrases).not.toContain(unsafe);
    }
    expect(phrases).toContain("plugin update");
    expect(phrases).toContain("플러그인 업데이트");
  });
});

describe("aggregatePerSkill (Phase 2)", () => {
  test("0 blocks → 0 unique skills", () => {
    const blocks = parseMainRs();
    const map = aggregatePerSkill(blocks);
    expect(map.size).toBe(0);
  });
});

describe("formatPhraseList", () => {
  test("Korean before English", () => {
    const out = formatPhraseList(["zebra", "사과", "apple", "배"]);
    expect(out).toBe('"배", "사과", "apple", "zebra"');
  });

  test("preserves quoting", () => {
    const out = formatPhraseList(["x", "y"]);
    expect(out).toMatch(/"x".*"y"/);
  });
});

describe("readSkillDescription", () => {
  test("recognizes 다음 표현에서 활성화: marker (deploy)", () => {
    const region = readSkillDescription(join(REPO_ROOT, "skills/deploy/SKILL.md"));
    expect(region).not.toBeNull();
    expect(region?.marker).toBe("다음 표현에서 활성화:");
    expect(region?.existingPhrases.length).toBeGreaterThan(0);
    expect(region?.existingPhrases).toContain("deploy");
  });

  test("recognizes alt marker for clarify (다음과 같은 불확실 컨텍스트에서 활성화:)", () => {
    const region = readSkillDescription(join(REPO_ROOT, "skills/clarify/SKILL.md"));
    expect(region).not.toBeNull();
    expect(region?.marker).toBe("다음과 같은 불확실 컨텍스트에서 활성화:");
  });

  test("parses YAML-escaped apostrophes in single-quoted descriptions", () => {
    const region = readSkillDescription(join(REPO_ROOT, "skills/whatsnew/SKILL.md"));
    expect(region).not.toBeNull();
    expect(region?.existingPhrases).toContain("what's new");
    expect(region?.existingPhrases.filter((phrase) => phrase === "what's new")).toHaveLength(1);
  });

  test("returns null for missing SKILL.md", () => {
    const region = readSkillDescription("/tmp/nonexistent-skill-xyz.md");
    expect(region).toBeNull();
  });
});

describe("computeDiff", () => {
  test("missing phrases are exactly main.rs - description", () => {
    const synthetic = readSkillDescription(join(REPO_ROOT, "skills/deploy/SKILL.md"))!;
    const mainRsPhrases = new Set(["deploy", "TOTALLY_NEW_PHRASE_FOR_TEST_xyz"]);
    const diff = computeDiff("deploy", synthetic, mainRsPhrases);
    expect(diff.missing).toContain("TOTALLY_NEW_PHRASE_FOR_TEST_xyz");
    expect(diff.missing).not.toContain("deploy"); // already in description
  });

  test("returns 0 missing when description already covers all", () => {
    const synthetic = readSkillDescription(join(REPO_ROOT, "skills/deploy/SKILL.md"))!;
    const subset = new Set(synthetic.existingPhrases.slice(0, 3));
    const diff = computeDiff("deploy", synthetic, subset);
    expect(diff.willAdd).toBe(0);
  });
});

describe("applyMerge idempotency", () => {
  test("second merge produces no changes", () => {
    const region = readSkillDescription(join(REPO_ROOT, "skills/deploy/SKILL.md"))!;
    const mainRsPhrases = new Set([...region.existingPhrases]); // exact match
    const merge = applyMerge(region, mainRsPhrases);
    expect(merge.changed).toBe(false);
  });

  test("first merge with new phrase changes; second does not", () => {
    const region = readSkillDescription(join(REPO_ROOT, "skills/deploy/SKILL.md"))!;
    const mainRsPhrases = new Set([...region.existingPhrases, "BRAND_NEW_PHRASE_xyz"]);
    const first = applyMerge(region, mainRsPhrases);
    expect(first.changed).toBe(true);

    // Simulate second merge by treating the merged result as new region
    const newRegion = {
      ...region,
      existingPhrases: first.finalPhrases,
    };
    const second = applyMerge(newRegion, mainRsPhrases);
    expect(second.changed).toBe(false);
  });
});

describe("end-to-end codegen idempotency", () => {
  test("running codegen --apply twice produces identical SKILL.md content", () => {
    // First apply (already done via prior phase 1 invocation; this test re-runs to confirm).
    const skillPath = join(REPO_ROOT, "skills/deploy/SKILL.md");
    const before = readFileSync(skillPath, "utf8");

    execFileSync("bun", ["run", "scripts/codegen-skill-keywords-from-rust.ts", "--apply"], {
      cwd: REPO_ROOT,
      encoding: "utf8",
    });

    const afterFirst = readFileSync(skillPath, "utf8");

    execFileSync("bun", ["run", "scripts/codegen-skill-keywords-from-rust.ts", "--apply"], {
      cwd: REPO_ROOT,
      encoding: "utf8",
    });

    const afterSecond = readFileSync(skillPath, "utf8");

    expect(afterFirst).toBe(afterSecond);
    // Also: second run should produce zero changes vs original (assuming caller already converged)
    expect(afterFirst).toBe(before);
  });
});


describe("generated SKILL frontmatter", () => {
  test("single-quoted description lines escape apostrophes for YAML", () => {
    const skillPath = join(REPO_ROOT, "skills/whatsnew/SKILL.md");
    const line = readFileSync(skillPath, "utf8").split("\n").find((l) => l.startsWith("description: "));
    expect(line).toBeDefined();
    expect(line).toMatch(/^description:\s*'(?:[^']|'')*'$/);
  });
});
