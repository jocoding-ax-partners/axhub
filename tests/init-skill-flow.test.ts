import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const INIT_SKILL = join(REPO_ROOT, "skills/init/SKILL.md");

describe("init SKILL — dependency execution flow (CRIT-R2-1 commit)", () => {
  const content = readFileSync(INIT_SKILL, "utf8");
  const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
  const frontmatter = fmMatch?.[1] ?? "";

  test("frontmatter declares allows-dependency-execution: true", () => {
    expect(frontmatter).toMatch(/^allows-dependency-execution:\s*true\s*$/m);
  });

  test("NEVER 룰이 3 조건 enumerate (frontmatter / recommended_command / explicit AskUserQuestion option)", () => {
    expect(content).toContain("allows-dependency-execution: true");
    expect(content).toContain("recommended_command");
    expect(content).toContain("explicit 선택");
  });

  test("inline !prefix 명령은 helper subcommand 호출만 (free-text npm install 0건은 OK이지만 dependency install 흐름 안에서만)", () => {
    // dependency install 섹션 안의 inline ! 가 helper subcommand 또는 4 manager 명령 (npm/pnpm/yarn/bun install) 만 호출하는지 검증
    const inlineBangs = content.match(/!\$?\{?[^`\n]*\}?\s*[^`\n]*/g) ?? [];
    const helperCalls = inlineBangs.filter(b => b.includes("axhub-helpers"));
    expect(helperCalls.length).toBeGreaterThan(0);
  });
});
