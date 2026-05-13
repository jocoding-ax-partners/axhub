import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const skillPath = join(process.cwd(), "skills", "deploy", "SKILL.md");

function deploySkill(): string {
  return readFileSync(skillPath, "utf8");
}

describe("deploy SKILL Step 1.6 — 3-way split invariants (issue #81 PR A2)", () => {
  test("Step 1.6 branches by IN_FLIGHT_COMMIT vs RESOLVE_COMMIT", () => {
    const body = deploySkill();
    expect(body).toContain("IN_FLIGHT_COMMIT");
    expect(body).toContain("RESOLVE_COMMIT");
    expect(body).toContain(".in_flight_deploy.commit_sha");
    expect(body).toContain(".resolve.commit_sha");
  });

  test("Step 1.6a (same-commit) question literal", () => {
    const body = deploySkill();
    expect(body).toContain("이미 배포가 진행 중이에요. 어떻게 할까요?");
  });

  test("Step 1.6b (cross-tenant) question literal", () => {
    const body = deploySkill();
    expect(body).toContain("다른 사람이 같은 앱에 배포 중일 가능성이 있어요. 어떻게 할까요?");
  });

  test("Step 1.6c (uncertain) question literal", () => {
    const body = deploySkill();
    expect(body).toContain("배포 중인 게 있는데 누구 건지 확인 중이에요. 어떻게 할까요?");
  });

  test("INFLIGHT_BRANCH variable carries 3-way decision", () => {
    const body = deploySkill();
    expect(body).toContain('INFLIGHT_BRANCH="uncertain"');
    expect(body).toContain('INFLIGHT_BRANCH="same"');
    expect(body).toContain('INFLIGHT_BRANCH="cross_tenant"');
  });

  test("Step 3.6 cross-references 3-way logic", () => {
    const body = deploySkill();
    expect(body).toContain("3-way 분기");
    expect(body).toContain("1.6a");
    expect(body).toContain("1.6b");
    expect(body).toContain("1.6c");
  });

  test("Step 4 POSIX grep uses anchored sub-key pattern", () => {
    const body = deploySkill();
    expect(body).toContain(
      "grep -qE '^axhub-error-sub-key:.*64:validation\\.deployment_in_progress'",
    );
  });

  test("Step 4 PowerShell Select-String uses anchored sub-key pattern", () => {
    const body = deploySkill();
    expect(body).toContain(
      "Select-String -Path $AxhubStderrTmp.FullName -Pattern '^axhub-error-sub-key:.*64:validation\\.deployment_in_progress' -Quiet",
    );
  });
});
