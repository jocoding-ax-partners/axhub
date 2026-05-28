import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const skillPath = join(process.cwd(), "skills", "verify", "SKILL.md");
const registryPath = join(
  process.cwd(),
  "tests",
  "fixtures",
  "ask-defaults",
  "registry.json",
);

function verifySkill(): string {
  return readFileSync(skillPath, "utf8");
}

function verifyRegistry(): any {
  return JSON.parse(readFileSync(registryPath, "utf8")).verify;
}

describe("verify SKILL Phase 26 invariants", () => {
  test("frontmatter declares multi-step preflight workflow", () => {
    const body = verifySkill();
    expect(body).toContain("name: verify");
    expect(body).toContain("multi-step: true");
    expect(body).toContain("needs-preflight: true");
    // Phase 27 (ADR-0013): preflight is now an in-body bash step, not a `!command`
    // injection. Assert the body invokes `axhub-helpers preflight --json` directly.
    expect(body).toContain("axhub-helpers");
    expect(body).toMatch(/preflight\s+--json/);
  });

  test("Step 0 renders TodoWrite checklist before evidence collection", () => {
    const body = verifySkill();
    const step0 = body.indexOf("0. **Render TodoWrite checklist");
    const identify = body.indexOf("1. **최근 배포 식별");
    expect(step0).toBeGreaterThan(-1);
    expect(identify).toBeGreaterThan(step0);
    for (const marker of ["최근 배포 식별", "axhub deploy status 호출", "axhub deploy logs 확인", "verdict 안내"]) {
      expect(body).toContain(marker);
    }
  });

  test("non-interactive AskUserQuestion guard has registry fallback", () => {
    const body = verifySkill();
    expect(body).toContain("Non-interactive AskUserQuestion guard");
    expect(body).toContain("CLAUDE_NON_INTERACTIVE");
    expect(body).toContain("헬스 endpoint 가 설정 안 돼 있어요");

    const registry = verifyRegistry();
    const entry = registry["헬스 endpoint 가 설정 안 돼 있어요. 지금 설정해서 더 깊게 검증할까요?"];
    expect(entry.safe_default).toBe("skip");
    expect(entry.allowed_safe_defaults).toContain("지금 설정");
  });

  test("verdict workflow is evidence based and exposes JSON automation", () => {
    const body = verifySkill();
    expect(body).toContain("axhub deploy status");
    expect(body).toContain("axhub deploy logs <DEPLOY_ID> --app <APP> --source pod");
    expect(body).toContain("ERROR");
    expect(body).toContain("FATAL");
    expect(body).toContain("axhub-helpers verify --json --app paydrop");
    expect(body).toContain("--app-id 도 alias");
    expect(body).toContain("✅ 라이브 확정");
    expect(body).toContain("⚠️ 의심");
    expect(body).toContain("❌ 라이브 안 됨");
  });
});
