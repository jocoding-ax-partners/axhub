// Phase 27 — preflight is now an IN-BODY bash step, not a load-time `!command`
// injection. ADR-0013 (supersedes ADR-0011): the `!`node -e ...`` injection
// hard-failed on first run because Claude Code permission-gates the outer
// `node -e` wrapper itself, and its inner denialRegex fallback could never catch
// its own denial (dead path). Every needs-preflight:true SKILL must carry the
// canonical in-body preflight block, and NO skill may carry the dead `!command`
// injection or leftover migration debris.

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
/**
 * In-body invocation = the canonical block's assignment signature, not a bare
 * `preflight --json` mention. A loose check would false-pass a skill that only
 * references preflight in a later/legacy step (e.g. deploy:388) even if its
 * upfront block were deleted. Matches scripts/preflight-block.ts CANONICAL_PREFLIGHT_BLOCK.
 */
const INVOKES_PREFLIGHT_RE = /PREFLIGHT_JSON=\$\("\$HELPER" preflight --json/;

/**
 * Migration debris left by an incomplete `!command`->in-body conversion. Each fragment
 * belongs to the retired injection era; none may survive in a migrated SKILL body.
 * (recover/apps shipped with an orphan ``` wrapper; axhub-diagnose with an orphan caption
 * — both passed skill-doctor/tone/structural-less gates and were caught by eye/adversarial
 * review. This locks the whole class.)
 */
const DEBRIS_FRAGMENTS = [
  "이 줄은", // leftover "이 줄은 ... 실행돼요/주입돼요" preprocessing caption
  "자동 주입", // "auto-inject" wording from the injection era
  "Pre-execute preflight context", // the deploy-variant injection header
  "Claude Code SKILL preprocessing", // template caption variant
];
/** Orphan ``` fence opening immediately before the canonical block prose (recover/apps class). */
const ORPHAN_FENCE_RE = /^```\n\*\*Preflight \(인증/m;

describe("Phase 27 — in-body preflight per needs-preflight frontmatter (ADR-0013)", () => {
  for (const slug of skillSlugs) {
    const { needsPreflight, content } = readFrontmatter(slug);

    if (needsPreflight) {
      test(`skills/${slug}/SKILL.md (needs-preflight: true) carries the canonical in-body preflight block, no !command injection`, () => {
        expect(INVOKES_PREFLIGHT_RE.test(content)).toBe(true);
        expect(INJECTION_RE.test(content)).toBe(false);
      });
    } else {
      test(`skills/${slug}/SKILL.md (needs-preflight: false) carries no !command preflight injection`, () => {
        expect(INJECTION_RE.test(content)).toBe(false);
      });
    }
  }

  // Regression lock for the incomplete-migration class (orphan captions + orphan fences).
  for (const slug of skillSlugs) {
    test(`skills/${slug}/SKILL.md carries no migration debris (orphan caption / fence)`, () => {
      const { content } = readFrontmatter(slug);
      for (const fragment of DEBRIS_FRAGMENTS) {
        expect(content.includes(fragment), `leftover injection-era fragment: "${fragment}"`).toBe(false);
      }
      expect(ORPHAN_FENCE_RE.test(content), "orphan ``` fence wrapping the canonical preflight block").toBe(false);
    });
  }

  test("at least 4 SKILLs are declared needs-preflight: true (Phase 18 baseline)", () => {
    const count = skillSlugs.filter((s) => readFrontmatter(s).needsPreflight).length;
    expect(count).toBeGreaterThanOrEqual(4);
  });
});
