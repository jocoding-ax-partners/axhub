// Phase 17 US-1705 — assert TodoWrite progress UI present in 5 multi-step SKILLs.
// If a future rewrite removes TodoWrite, vibe coder loses the real-time progress
// panel and the SKILL silently regresses on the C2/US-1702 contract.

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");

const TODOWRITE_SKILLS = ["deploy", "recover", "update", "upgrade", "doctor"] as const;

describe("Phase 17 C2/US-1702 — TodoWrite presence in 5 multi-step SKILLs", () => {
  for (const slug of TODOWRITE_SKILLS) {
    test(`skills/${slug}/SKILL.md contains TodoWrite call with todos array`, () => {
      const content = readFileSync(join(REPO_ROOT, `skills/${slug}/SKILL.md`), "utf8");
      expect(content).toContain("TodoWrite({ todos: [");
      // Minimum 4 todos per multi-step skill
      const todoCount = (content.match(/\{ content:/g) ?? []).length;
      expect(todoCount).toBeGreaterThanOrEqual(4);
    });

    test(`skills/${slug}/SKILL.md TodoWrite has activeForm in 해요체 (no 합니다)`, () => {
      const content = readFileSync(join(REPO_ROOT, `skills/${slug}/SKILL.md`), "utf8");
      // Extract activeForm strings from TodoWrite block
      const activeFormMatches = content.match(/activeForm:\s*"([^"]+)"/g) ?? [];
      expect(activeFormMatches.length).toBeGreaterThan(0);
      for (const af of activeFormMatches) {
        expect(af).not.toMatch(/합니다|입니다|시겠어요|드립니다|당신|아이고/);
      }
    });
  }
});
