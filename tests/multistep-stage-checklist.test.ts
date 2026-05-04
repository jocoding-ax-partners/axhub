import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const SKILLS_DIR = join(REPO_ROOT, "skills");

const skillSlugs = readdirSync(SKILLS_DIR).filter((slug) => {
  try {
    readFileSync(join(SKILLS_DIR, slug, "SKILL.md"), "utf8");
    return true;
  } catch {
    return false;
  }
});

const isMultiStep = (content: string) => /^multi-step:\s*true\s*$/m.test(content);

const todoContents = (content: string): string[] =>
  [...content.matchAll(/content:\s*"([^"]+)"/g)].map((m) => m[1]).filter(Boolean);

describe("multi-step skills show user-facing stage checklists", () => {
  for (const slug of skillSlugs) {
    test(`skills/${slug}/SKILL.md mirrors TodoWrite as a visible 작업 단계 checklist`, () => {
      const content = readFileSync(join(SKILLS_DIR, slug, "SKILL.md"), "utf8");
      if (!isMultiStep(content)) {
        expect(true).toBe(true);
        return;
      }

      expect(content).toContain("작업 단계");
      for (const todo of todoContents(content)) {
        expect(content, `${slug} missing visible checkbox for ${todo}`).toContain(`□ ${todo}`);
      }
    });
  }
});
