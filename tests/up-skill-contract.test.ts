// up 스킬 본문 계약 — 로컬 소스 배포 lane 의 acceptance example 을 문자열로 잠가요.
//
// 라우팅 자체는 LLM 판정이라 assert 할 수 없지만, 본문이 그 판정에 필요한
// 지시를 실제로 들고 있는지는 잠글 수 있어요. `tests/plugins-skill-contract.test.ts`
// 가 10번째 skill 에 쓴 것과 같은 모양이에요.

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (relative: string): string => readFileSync(join(REPO_ROOT, relative), "utf8");

describe("up skill contract", () => {
  const body = readRepo("skills/up/SKILL.md");
  const details = readRepo("skills/up/references/workflow-details.md");

  // AE1 — dirty 작업 트리 + 저장소 미연결 앱이 이 lane 의 정상 입력이에요.
  // deploy 가 첫 명령에서 exit 64 로 끊던 조합이라, 그 명령을 쓰지 않는다는
  // 금지와 커밋 게이트 부재를 함께 잠가요.
  test("AE1: 커밋 상태를 게이트로 쓰지 않고 커밋 게이트 명령을 금지해요", () => {
    expect(body).toContain("커밋 상태를 게이트로 쓰지 않아요");
    // 방향성 있는 금지문 — 이름 존재만 보면 "deploy-preview-summary 로 시작해요"
    // 로 뒤집혀도 통과해요.
    expect(body).toContain("`axhub plugin-support deploy-preview-summary` / `deploy-approved-run` 호출 금지");
    expect(body).toContain("`axhub plugin-support deploy-preview-summary` 와 `deploy-approved-run` 은 **쓰지 않아요.**");
    expect(body).toContain("plugin-support deploy-prep --intent deploy --json");
    // 커밋을 만들거나 push 하지 않는다는 계약.
    expect(body).toContain("커밋을 만들거나 push 하지 않");
  });

  // AE2 — 구 CLI 는 update 로 보내고 deploy create 로 대체하지 않아요.
  test("AE2: 구 CLI unknown-command 는 update 로 보내고 deploy create 로 대체하지 않아요", () => {
    expect(body).toContain("0.29.0");
    expect(body).toContain("unknown command");
    expect(body).toMatch(/`update` 로 보내고 멈춰요/);
    expect(body).toContain("`axhub deploy create` 로 대체하지 않아요");
  });

  // AE3 — headless 안전 기본값은 dry-run 이고 실행으로 넘어가지 않아요.
  test("AE3: headless 안전 기본값은 dry-run 이에요", () => {
    expect(body).toContain("Headless 안전 기본값은 dry-run 이에요");
    expect(body).toContain("`--execute` 를 실행하지 않아요");
  });

  // 성공 선언은 verify 단독이에요 (AP-1).
  test("성공 선언은 deploy verify 단독으로만 해요", () => {
    expect(body).toContain("axhub deploy verify");
    expect(body).toContain("verify 전 성공 선언 금지");
    // AP-16 폴링 예산이 verify 루프에 걸려 있어요.
    expect(body).toContain("폴링 예산");
    expect(body).toContain("최대 30회 또는 10분");
    expect(body).toContain("--wait --wait-interval 20s --wait-timeout 10m");
  });

  // Silent-pass 차단 — 2단계 명령의 --dry-run 을 --execute 로 뒤집으면 승인 전
  // 실배포가 되는데, 문자열 존재 assert 만으로는 그 변조가 통과해요.
  test("미리보기는 --dry-run, 실행은 --execute 로 분리돼 있어요", () => {
    const preview = "axhub up --app \"$APP_ID\" --path . --dry-run --json";
    const execute = "axhub up --app \"$APP_ID\" --path . --execute";
    expect(body).toContain(preview);
    expect(body).toContain(execute);
    // 승인 절이 실행 명령보다 앞에 와야 해요.
    const approvalAt = body.indexOf("## 3단계 — 승인");
    const executeAt = body.indexOf(execute);
    expect(approvalAt).toBeGreaterThan(-1);
    expect(executeAt).toBeGreaterThan(approvalAt);
    // 승인 절 앞에는 --execute 를 쓰는 명령이 없어야 해요. 산문에서 --execute 를
    // 언급하는 것(재패킹 경고)은 허용하고, 실행 명령만 금지해요.
    const beforeApproval = body.slice(0, approvalAt);
    expect(beforeApproval.match(/axhub up [^\n]*--execute/)).toBeNull();
  });

  // dry-run 패킹 실패는 preview 를 만들지 않고 멈춰요.
  test("dry-run 패킹 실패는 preview 없이 멈춰요", () => {
    expect(body).toContain("dry-run 이 실패하면 preview 카드를 만들지 않아요");
    expect(body).toContain("파일 0개");
  });

  // in-flight 배포 가드 — deploy-preview-summary 를 건너뛰면서 잃지 않아요.
  test("in-flight 배포를 만나면 묻고 멈춰요", () => {
    expect(body).toContain("in_flight_deploy");
    expect(body).toContain("이미 진행 중인 배포가 있어요");
  });

  // static 앱은 성공 선언 경로가 달라 deploy 로 양보해요.
  test("static 앱은 deploy 의 static lane 으로 양보해요", () => {
    expect(body).toContain("deploy_method");
    expect(body).toContain("static lane");
  });

  // 양보 계약 — deploy·bootstrap 이 업로드 명령을 직접 실행하지 않아야 해요.
  // 이 삭제를 잠그지 않으면 두 스킬이 같은 배포를 각자 승인·실행할 수 있어요.
  test("deploy·bootstrap 은 업로드 명령을 직접 실행하지 않아요", () => {
    for (const path of [
      "skills/deploy/SKILL.md",
      "skills/deploy/references/workflow-details.md",
      "skills/bootstrap/SKILL.md",
      "skills/bootstrap/references/github-blocked-local-deploy.md",
    ]) {
      const text = readRepo(path);
      expect(text, path).not.toMatch(/axhub up [^\n]*--execute/);
      expect(text, path).not.toMatch(/axhub up [^\n]*--dry-run/);
    }
  });

  // reference 는 참고용이고 실행 지시를 본문 밖으로 빼지 않아요 (KTD5).
  test("reference 는 본문이 실행에 필요한 전부를 담는다고 밝혀요", () => {
    expect(details).toContain("SKILL.md 본문이 실행에 필요한 전부를 담아요");
  });
});
