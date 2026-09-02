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
  const details = read("skills/deploy/references/workflow-details.md");
  const language = read("skills/deploy/references/user-facing-language.md");
  const importSkill = read("skills/import/SKILL.md");
  const up = read("skills/up/SKILL.md");

  test("deploy: apps get 에서 staging_enabled 를 읽어 STAGING_ENABLED 로 잡아요", () => {
    expect(deploy).toContain("`git_backend.source`, and `staging_enabled` only");
    expect(deploy).toContain("Bind `STAGING_ENABLED` from `staging_enabled`");
  });

  test("deploy: preview 환경 라벨은 staging_enabled 로 운영/스테이징을 갈라요", () => {
    expect(deploy).toContain("`운영` when false, `스테이징` when true");
    // 옛 규칙(무조건 `운영`)이 되살아나면 실패해요.
    expect(deploy).not.toContain("Display the environment as `운영`, not `prod`");
    expect(details).toContain("`운영` when `STAGING_ENABLED` is false and `스테이징` when it is true");
    expect(details).not.toContain("The visible environment label is `운영`;");
    expect(language).toContain("`스테이징` for a staging opt-in app (`staging_enabled=true`");
  });

  test("deploy: verify exit 0 은 environment 로 갈라 스테이징이면 심사 신청을 안내해요", () => {
    for (const body of [deploy, details]) {
      expect(body).toContain("운영에는 아직 반영되지 않았");
      expect(body).toContain('axhub publish --app "$APP_ID" --deployment-id "$DEPLOY_ID" --execute');
      // null/부재는 운영이 아니라 "알 수 없음" — STAGING_ENABLED 로 되돌아가요.
      expect(body).toMatch(/`environment` 가 없거나 null 이면[^\n]*운영으로 간주하지 않/);
    }
    expect(deploy).toContain("never `운영 반영 완료`");
    expect(deploy).toContain("심사 신청은 사용자가 요청할 때만 실행해요");
  });

  test("import: 성공 안내가 environment=staging 을 운영 반영으로 말하지 않아요", () => {
    expect(importSkill).toContain("최종 `deploy verify --json` 의 `environment` 를 먼저 읽어요");
    expect(importSkill).toContain("스테이징에만 반영됐고 운영에는 아직 반영되지 않았다고 같은 성공 블록에서 말하고");
    expect(importSkill).toContain("axhub publish --app <app> --deployment-id <deployment-id> --execute");
    expect(importSkill).toContain('"운영 반영 완료" 라고 말하지 않고');
    expect(importSkill).toMatch(/`environment` 가 없거나 null 이면[^\n]*`staging_enabled` 를 한 번 읽어/);
  });

  test("up: 환경 라벨과 verify 성공 요약이 같은 규칙을 따라요", () => {
    expect(up).toContain("`deploy_method`, `staging_enabled`만 읽어요");
    expect(up).toContain("`STAGING_ENABLED` 가 false 면 `운영`, true 면 `스테이징`");
    expect(up).not.toContain("환경은 `운영` 으로 표시하고");
    expect(up).toContain('axhub publish --app "$APP_ID" --deployment-id "$DEPLOY_ID" --execute');
    expect(up).toMatch(/`environment` 가 없거나 null 이면 `STAGING_ENABLED` 로 판단하고 운영으로 간주하지 않아요/);
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
