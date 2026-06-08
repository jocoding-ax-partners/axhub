import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const skillPath = join(process.cwd(), "skills", "deploy", "SKILL.md");

function deploySkill(): string {
  return readFileSync(skillPath, "utf8");
}

describe("deploy SKILL Step 1.7 — status-first gate (create is the fallback)", () => {
  test("Step 1.7 section is present and runs before the preview card", () => {
    const body = deploySkill();
    const step1_7 = body.indexOf("Step 1.7");
    const preview = body.indexOf("Render preview card");
    expect(step1_7).toBeGreaterThan(-1);
    expect(preview).toBeGreaterThan(-1);
    expect(step1_7).toBeLessThan(preview);
  });

  test("Step 1.7 reads github_connected and in_flight_deploy.id from deploy-prep", () => {
    const body = deploySkill();
    expect(body).toContain("[deploy:Step 1.7 status-first] entered");
    expect(body).toContain(".github_connected");
    expect(body).toContain(
      'STATUS_FIRST_ID=$(echo "$DEPLOY_PREP_JSON" | jq -r \'.in_flight_deploy.id // ""\')',
    );
  });

  test("an already-running deploy is watched, not re-created (no consent / no create)", () => {
    const body = deploySkill();
    expect(body).toContain("새 `deploy create` 를 실행하지 않아요");
    expect(body).toContain("status 보고 배포가 아니면 그제서야 진짜 create");
  });

  test("status-first preserves same-commit vs cross-tenant discrimination", () => {
    const body = deploySkill();
    expect(body).toContain(".in_flight_deploy.commit_sha");
    expect(body).toContain(".resolve.commit_sha");
    expect(body).toContain("same-commit 일 때만 자동 watch");
  });

  test("status-first polling is skipped in non-interactive contexts", () => {
    const body = deploySkill();
    expect(body).toContain('[ -t 1 ] && [ -z "$CI" ] && [ -z "$CLAUDE_NON_INTERACTIVE" ]');
  });

  test("the create path is labelled the fallback reached only after status-first", () => {
    const body = deploySkill();
    expect(body).toContain("fallback create 경로");
  });

  test("status-first honors a Step 1.6 force_new choice (no re-routing the same in-flight to watch)", () => {
    const body = deploySkill();
    expect(body).toContain("Step 1.6 이 이미 in-flight 를 처리했으면");
    expect(body).toContain("같은 in-flight 를 다시 watch 로 되돌리지 않아요");
  });
});
