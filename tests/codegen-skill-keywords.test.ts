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

describe("parseMainRs", () => {
  const blocks = parseMainRs();

  test("extracts 19 keyword blocks from main.rs (18 contains_any + 1 if-eq single-token)", () => {
    expect(blocks.length).toBe(19);
  });

  test("each block has at least 1 phrase", () => {
    for (const block of blocks) {
      expect(block.phrases.length).toBeGreaterThan(0);
    }
  });

  test("deploy block contains expected core phrases", () => {
    const deployBlock = blocks.find((b) => b.skill === "deploy");
    expect(deployBlock).toBeDefined();
    expect(deployBlock?.phrases).toContain("deploy");
    expect(deployBlock?.phrases).toContain("배포");
    expect(deployBlock?.phrases).toContain("ship");
  });

  test("single-token if p == case captured (clarify env)", () => {
    const clarifyBlocks = blocks.filter((b) => b.skill === "clarify");
    expect(clarifyBlocks.length).toBe(2); // env clarify + axhub clarify
    const allClarifyPhrases = clarifyBlocks.flatMap((b) => b.phrases);
    expect(allClarifyPhrases).toContain("환경"); // single-token if p == "환경"
    expect(allClarifyPhrases).toContain("axhub"); // axhub clarify
  });

  test("upgrade compound guard does not become standalone plugin/update triggers", () => {
    const upgradeBlock = blocks.find((b) => b.skill === "upgrade");
    expect(upgradeBlock).toBeDefined();
    const phrases = upgradeBlock?.phrases ?? [];
    for (const unsafe of ["plugin", "플러그인", "update", "upgrade", "version", "업데이트", "업그레이드", "버전", "새 버전", "호환"]) {
      expect(phrases).not.toContain(unsafe);
    }
    expect(phrases).toContain("plugin update");
    expect(phrases).toContain("플러그인 업데이트");
  });
});

describe("aggregatePerSkill", () => {
  test("merges multiple blocks for same skill into one set", () => {
    const blocks = parseMainRs();
    const map = aggregatePerSkill(blocks);
    expect(map.size).toBe(18); // unique skill names

    const clarifySet = map.get("clarify");
    expect(clarifySet?.has("환경")).toBe(true);
    expect(clarifySet?.has("axhub")).toBe(true);
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
