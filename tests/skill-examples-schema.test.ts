// Phase 9 AC traceability shim — validates frontmatter examples across active skills.

import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { computeExamplesIssues } from "../scripts/skill-doctor-quality";

const root = join(import.meta.dir, "..");
const skillsDir = join(root, "skills");
const slugs = readdirSync(skillsDir).filter((slug) => {
  try {
    readFileSync(join(skillsDir, slug, "SKILL.md"), "utf8");
    return slug !== "_template";
  } catch {
    return false;
  }
});

describe("Phase 9 examples schema", () => {
  test("all active skills have valid examples frontmatter", () => {
    expect(slugs.length).toBeGreaterThanOrEqual(18);
    for (const slug of slugs) {
      const content = readFileSync(join(skillsDir, slug, "SKILL.md"), "utf8");
      const fm = content.match(/^---\n([\s\S]*?)\n---/)?.[1] ?? "";
      expect(computeExamplesIssues(slug, fm), slug).toEqual([]);
    }
  });
});
