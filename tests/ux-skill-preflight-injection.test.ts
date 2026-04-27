// Phase 18 R2 / US-1804 — assert !command preflight injection present in every
// SKILL declared `needs-preflight: true` in frontmatter. New SKILL with
// needs-preflight: true → must include `!`...preflight --json`` literal at
// workflow start or test FAIL. Eliminates scaffold-rot.

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

const readFrontmatter = (slug: string): { needsPreflight: boolean; content: string } => {
  const content = readFileSync(join(SKILLS_DIR, slug, "SKILL.md"), "utf8");
  const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
  const fm = fmMatch?.[1] ?? "";
  const needsPreflight = /^needs-preflight:\s*true\s*$/m.test(fm);
  return { needsPreflight, content };
};

describe("Phase 18 R2/US-1804 — !command preflight injection per needs-preflight frontmatter", () => {
  for (const slug of skillSlugs) {
    const { needsPreflight, content } = readFrontmatter(slug);

    if (needsPreflight) {
      test(`skills/${slug}/SKILL.md (needs-preflight: true) contains !command preflight literal`, () => {
        // Match literal: !`${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers preflight --json`
        // Use substring check to avoid regex escaping headaches with backticks + braces.
        expect(content.includes("axhub-helpers preflight --json")).toBe(true);
        expect(content.includes("${CLAUDE_PLUGIN_ROOT}/bin/")).toBe(true);
      });
    } else {
      test(`skills/${slug}/SKILL.md (needs-preflight: false) — preflight injection not required`, () => {
        expect(true).toBe(true);
      });
    }
  }

  test("at least 4 SKILLs are declared needs-preflight: true (Phase 18 baseline)", () => {
    const count = skillSlugs.filter((s) => readFrontmatter(s).needsPreflight).length;
    expect(count).toBeGreaterThanOrEqual(4);
  });
});
