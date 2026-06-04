// Phase 17 US-1705 + Phase 18 R1 — assert TodoWrite progress UI present in
// every SKILL declared `multi-step: true` in frontmatter (not hardcoded list).
// New SKILL with multi-step: true → must add TodoWrite or test FAIL. Removes
// scaffold-rot — adding a SKILL no longer requires editing this test.

import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const SKILLS_DIR = join(REPO_ROOT, "skills");

const skillSlugs = readdirSync(SKILLS_DIR).filter((d) => {
  try {
    readFileSync(join(SKILLS_DIR, d, "SKILL.md"), "utf8");
    return true;
  } catch {
    return false;
  }
});

const readFrontmatter = (slug: string): { multiStep: boolean; content: string } => {
  const content = readFileSync(join(SKILLS_DIR, slug, "SKILL.md"), "utf8");
  const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
  const fm = fmMatch?.[1] ?? "";
  const multiStep = /^multi-step:\s*true\s*$/m.test(fm);
  return { multiStep, content };
};

describe("Phase 17 C2/US-1702 + Phase 18 R1 — TodoWrite presence per multi-step frontmatter", () => {
  for (const slug of skillSlugs) {
    const { multiStep, content } = readFrontmatter(slug);

    if (multiStep) {
      test(`skills/${slug}/SKILL.md (multi-step: true) contains TodoWrite call`, () => {
        expect(content).toContain("TodoWrite({ todos: [");
        const todoCount = (content.match(/\{ content:/g) ?? []).length;
        expect(todoCount).toBeGreaterThanOrEqual(4);
      });

      test(`skills/${slug}/SKILL.md updates TodoWrite status after progress`, () => {
        expect(content).toContain("TodoWrite status sync");
        expect(content).toContain("after every AskUserQuestion answer");
        expect(content).toContain("Do not leave the initial Step 0 list stale");
      });

      test(`skills/${slug}/SKILL.md silently skips TodoWrite when unavailable in Desktop`, () => {
        expect(content).toContain("TodoWrite availability");
        expect(content).toContain("available tool list");
        expect(content).toContain("do not call TodoWrite");
        expect(content).toContain("do not mention progress UI availability");
        expect(content).not.toContain("TodoWrite 없음");
        expect(content).not.toContain("TodoWrite unavailable");
        expect(content).not.toContain("TodoWrite skipped");
      });

      test(`skills/${slug}/SKILL.md TodoWrite activeForm in 해요체 (no forbidden tokens)`, () => {
        const activeFormMatches = content.match(/activeForm:\s*"([^"]+)"/g) ?? [];
        expect(activeFormMatches.length).toBeGreaterThan(0);
        for (const af of activeFormMatches) {
          expect(af).not.toMatch(/합니다|입니다|시겠어요|드립니다|당신|아이고/);
        }
      });
    } else {
      test(`skills/${slug}/SKILL.md (multi-step: false) — TodoWrite not required`, () => {
        // No assertion; declares the SKILL exempt. Future single-step SKILLs
        // pass without adding TodoWrite.
        expect(true).toBe(true);
      });
    }
  }

  test("at least 5 SKILLs are declared multi-step: true (Phase 18 baseline)", () => {
    const count = skillSlugs.filter((s) => readFrontmatter(s).multiStep).length;
    expect(count).toBeGreaterThanOrEqual(5);
  });

  test("skill scaffold preserves TodoWrite status sync instructions for future multi-step skills", () => {
    const scaffold = readFileSync(join(REPO_ROOT, "scripts", "skill-new.ts"), "utf8");
    const template = readFileSync(join(SKILLS_DIR, "_template", "SKILL.md.tmpl"), "utf8");

    expect(scaffold).toContain("TodoWrite status sync");
    expect(scaffold).toContain("TodoWrite availability");
    expect(scaffold).toContain("after every AskUserQuestion answer");
    expect(template).toContain("TodoWrite status sync");
    expect(template).toContain("TodoWrite availability");
  });
});
