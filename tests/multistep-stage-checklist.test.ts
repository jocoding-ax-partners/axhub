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

// Native Claude Code TodoWrite UI 가 진행 상황을 orange 박스로 보여주므로
// SKILL 본문에 markdown "작업 단계:" 체크리스트를 중복 출력하지 않아요.
// deploy SKILL 만 git-init nested checklist (sentinel `1.5. **Git 저장 지점 준비**` 이후)
// 를 별도 UX 흐름으로 허용해요.
describe("multi-step skills do not duplicate TodoWrite as markdown stage checklist", () => {
  for (const slug of skillSlugs) {
    test(`skills/${slug}/SKILL.md keeps stage progress in TodoWrite tool call only`, () => {
      const content = readFileSync(join(SKILLS_DIR, slug, "SKILL.md"), "utf8");
      if (!isMultiStep(content)) {
        expect(true).toBe(true);
        return;
      }

      if (slug === "deploy") {
        const parts = content.split("1.5. **Git 저장 지점 준비**");
        expect(
          parts.length,
          "deploy sentinel heading missing — update split key in this test"
        ).toBe(2);
        const mainStage = parts[0];
        expect(mainStage).not.toContain("작업 단계");
        expect(mainStage).not.toMatch(/^\s*(?:└\s*)?□\s/m);
        return;
      }

      expect(content).not.toContain("작업 단계");
      expect(content).not.toMatch(/^\s*(?:└\s*)?□\s/m);
    });
  }
});
