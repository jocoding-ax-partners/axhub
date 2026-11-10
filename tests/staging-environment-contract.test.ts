// AP-25 — 스테이징 옵트인 앱의 배포를 운영 반영으로 오인하지 않아요.
//
// 제보: `import --mode execute` 뒤 `deploy verify --wait` 가 status=succeeded ·
// success=true 만 돌려줘서, 스테이징에만 반영된 배포를 운영 반영 완료로 판단했어요.
// 스킬 본문이 (1) `apps get` 의 `staging_enabled` 로 환경 라벨을 가르고
// (2) verify JSON 의 `environment` 로 성공 요약을 갈라 다음 단계(심사 신청)를
// 안내하는 지시를 실제로 들고 있는지 문자열로 잠가요. 라우팅 자체는 LLM
// 판정이라 assert 할 수 없지만, 지시의 존재와 방향은 잠글 수 있어요.

import { describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const read = (relative: string): string => readFileSync(join(REPO_ROOT, relative), "utf8");

describe("staging environment contract (AP-25)", () => {
  const deploy = read("skills/deploy/SKILL.md");
  const importSkill = read("skills/import/SKILL.md");

  // 2026-11 deploy 개편(옛 up 을 합친 판): 저장 위치·정적 사이트별 길과 배포 번호는 CLI `deploy push` 가 맡고,
  // 스킬은 배포 결과의 `environment` 로 스테이징을 갈라 알려요. `apps get` 의 `staging_enabled`·`STAGING_ENABLED`
  // 라벨, 심사 신청 명령 안내, static lane 의 `staging_required` 분기(#909)는 그 개편 때 옛 본문과 함께 뺐어요.
  test("deploy: 결과의 environment 가 staging 이면 운영은 심사 뒤라고 알려요", () => {
    expect(deploy).toContain("`environment` 가 `staging` 이면 스테이징에만 반영됐고 운영은 심사 뒤라고 알려요");
    expect(deploy).toContain("| 주소 | `url` (스테이징이면 운영 반영은 심사 뒤) |");
    expect(deploy).toContain("확인하지 못한 항목은 됐다고 표시하지 않아요");
    expect(deploy).not.toContain("운영 반영 완료");
  });

  // 2026-11 import 개편: 결과 표의 주소 줄이 스테이징이면 운영 반영은 심사 뒤라고 적어요.
  // `environment` 분기·심사 신청 명령 안내는 그 개편 때 뺐어요 (AP-25 정책의 import 줄 참고).
  test("import: 결과 표가 스테이징 배포를 운영 반영으로 말하지 않아요", () => {
    expect(importSkill).toContain("| 주소 | `apps get` 의 `access_url` (스테이징이면 운영 반영은 심사 뒤) |");
    expect(importSkill).toContain("확인하지 못한 항목은 됐다고 표시하지 않아요");
  });

  test("fixture shim: staging verify 케이스가 environment=staging 과 review_request 를 내요", () => {
    const caseDir = mkdtempSync(join(tmpdir(), "axhub-staging-shim-"));
    const result = Bun.spawnSync({
      cmd: [join(REPO_ROOT, "tests/e2e/claude-cli/fixtures/bin/axhub"), "deploy", "verify", "dep-1", "--app", "paydrop", "--json"],
      env: { ...process.env, SHIM_CASE_DIR: caseDir, AXHUB_FIXTURE_VERIFY: "staging" },
    });
    expect(result.exitCode).toBe(0);
    const report = JSON.parse(Buffer.from(result.stdout).toString("utf8")) as Record<string, unknown>;
    expect(report.success).toBe(true);
    expect(report.environment).toBe("staging");
    expect(report.staging_enabled).toBe(true);
    expect(report.production_live).toBe(false);
    expect(report.next_step).toBe("review_request");
  });
});
