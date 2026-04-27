// Phase 17 US-1705 — assert every AskUserQuestion JSON block has a header
// field ≤12 Korean chars. UI overflow regression lock.

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

const koreanLength = (s: string): number => Array.from(s).length;

describe("Phase 17 C3/US-1703 — AskUserQuestion header ≤12 chars", () => {
  for (const slug of skillSlugs) {
    test(`skills/${slug}/SKILL.md AskUserQuestion headers ≤12 Korean chars`, () => {
      const content = readFileSync(join(SKILLS_DIR, slug, "SKILL.md"), "utf8");
      const headerMatches = content.match(/"header":\s*"([^"]+)"/g) ?? [];
      for (const m of headerMatches) {
        const value = m.match(/"header":\s*"([^"]+)"/)?.[1] ?? "";
        expect(koreanLength(value)).toBeLessThanOrEqual(12);
      }
    });
  }

  test("at least 8 SKILLs have a header field (Phase 17 C3 polish coverage)", () => {
    let count = 0;
    for (const slug of skillSlugs) {
      const content = readFileSync(join(SKILLS_DIR, slug, "SKILL.md"), "utf8");
      if (content.includes('"header":')) count++;
    }
    expect(count).toBeGreaterThanOrEqual(8);
  });
});
