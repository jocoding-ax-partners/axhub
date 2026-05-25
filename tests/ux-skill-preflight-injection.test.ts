// Phase 27 — preflight is now an IN-BODY bash step, not a load-time `!command`
// injection. ADR-0013 (supersedes ADR-0011): the `!`node -e ...`` injection
// hard-failed on first run because Claude Code permission-gates the outer
// `node -e` wrapper itself, and its inner denialRegex fallback could never catch
// its own denial (dead path). Every needs-preflight:true SKILL must invoke
// `axhub-helpers preflight --json` in its workflow body, and NO skill may carry
// the dead `!command` injection.

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

/** The retired load-time injection: `!`node -e "...axhub-helpers...preflight..."`` */
const INJECTION_RE = /^!`node -e "[^\n]*axhub-helpers[^\n]*preflight[^\n]*"`$/m;
/** In-body invocation signal: `preflight --json` (helper is picked into `$HELPER` first). */
const INVOKES_PREFLIGHT_RE = /preflight\s+--json/;

describe("Phase 27 — in-body preflight per needs-preflight frontmatter (ADR-0013)", () => {
  for (const slug of skillSlugs) {
    const { needsPreflight, content } = readFrontmatter(slug);

    if (needsPreflight) {
      test(`skills/${slug}/SKILL.md (needs-preflight: true) invokes axhub-helpers preflight in-body, no !command injection`, () => {
        expect(INVOKES_PREFLIGHT_RE.test(content)).toBe(true);
        expect(INJECTION_RE.test(content)).toBe(false);
      });
    } else {
      test(`skills/${slug}/SKILL.md (needs-preflight: false) carries no !command preflight injection`, () => {
        expect(INJECTION_RE.test(content)).toBe(false);
      });
    }
  }

  test("at least 4 SKILLs are declared needs-preflight: true (Phase 18 baseline)", () => {
    const count = skillSlugs.filter((s) => readFrontmatter(s).needsPreflight).length;
    expect(count).toBeGreaterThanOrEqual(4);
  });
});
