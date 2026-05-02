// Phase 17 US-1705 — D1 sentinel presence lint (CHEAP, INSUFFICIENT ALONE per
// Critic round 2). Every SKILL with an AskUserQuestion call site MUST contain
// the literal sentinel `Non-interactive AskUserQuestion guard (D1)`. Pairs with
// extended live-plugin-smoke.sh subprocess test for actual hang regression.

import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const SKILLS_DIR = join(REPO_ROOT, "skills");

const SENTINEL = "Non-interactive AskUserQuestion guard (D1)";

const skillSlugs = readdirSync(SKILLS_DIR).filter((d) => {
  try {
    readFileSync(join(SKILLS_DIR, d, "SKILL.md"), "utf8");
    return true;
  } catch {
    return false;
  }
});

describe("Phase 23 — D1 fallback sentinel in all 17 SKILLs", () => {
  for (const slug of skillSlugs) {
    test(`skills/${slug}/SKILL.md contains D1 sentinel`, () => {
      const content = readFileSync(join(SKILLS_DIR, slug, "SKILL.md"), "utf8");
      expect(content).toContain(SENTINEL);
    });

    test(`skills/${slug}/SKILL.md D1 block references registry path`, () => {
      const content = readFileSync(join(SKILLS_DIR, slug, "SKILL.md"), "utf8");
      expect(content).toContain("tests/fixtures/ask-defaults/registry.json");
    });
  }

  test("exactly 17 SKILLs have the sentinel (no drop)", () => {
    let count = 0;
    for (const slug of skillSlugs) {
      const content = readFileSync(join(SKILLS_DIR, slug, "SKILL.md"), "utf8");
      if (content.includes(SENTINEL)) count++;
    }
    expect(count).toBe(17);
  });
});
