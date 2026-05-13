import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const INIT_SKILL = join(REPO_ROOT, "skills/init/SKILL.md");

describe("init SKILL workflow — top-level step numbering uniqueness (Plan F′ regression guard)", () => {
  const content = readFileSync(INIT_SKILL, "utf8");

  function extractWorkflowBody(): string {
    const startMarker = "## Workflow";
    const endMarker = "## NEVER";
    const startIdx = content.indexOf(startMarker);
    const endIdx = content.indexOf(endMarker);
    if (startIdx < 0 || endIdx < 0 || endIdx <= startIdx) {
      throw new Error("Workflow / NEVER markers not found in init SKILL");
    }
    return content.slice(startIdx, endIdx);
  }

  test("Steps 0..6 each appear exactly once at top-level (no `5.` collisions)", () => {
    const body = extractWorkflowBody();

    const stepHeaders: number[] = [];
    const re = /^(\d+)\.\s+\*\*/gm;
    let m: RegExpExecArray | null;
    while ((m = re.exec(body)) !== null) {
      const n = parseInt(m[1], 10);
      if (!Number.isNaN(n)) stepHeaders.push(n);
    }

    const sorted = [...stepHeaders].sort((a, b) => a - b);
    expect(sorted).toEqual([0, 1, 2, 3, 4, 5, 6]);
    expect(new Set(sorted).size).toBe(sorted.length);
  });

  test("Step 6 (결과 안내) renders closed-form 5-step block with backend-truth labels", () => {
    expect(content).toContain("6. **결과와 다음 액션을 안내해요.**");
    expect(content).toContain("GitHub 연결 (배포에 꼭 필요해요)");
    expect(content).not.toMatch(/3\.\s*GitHub 연결\s*\(선택\)/);
  });

  test("dependency-install subsection uses D-prefix (D1..D5) to avoid top-level collision", () => {
    expect(content).toContain("D1. plan 을 조회해요");
    expect(content).toContain("D2. AskUserQuestion");
    expect(content).toContain("D3. 사용자가 `inline_session`");
    expect(content).toContain("D4. 실행 후 verify");
    expect(content).toContain("D5. 실패 시 에러 분류");
  });
});
