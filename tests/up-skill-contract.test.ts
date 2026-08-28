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
    expect(body).toContain("deploy-preview-summary");
    expect(body).toContain("deploy-approved-run");
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

  // reference 는 참고용이고 실행 지시를 본문 밖으로 빼지 않아요 (KTD5).
  test("reference 는 본문이 실행에 필요한 전부를 담는다고 밝혀요", () => {
    expect(details).toContain("SKILL.md 본문이 실행에 필요한 전부를 담아요");
  });
});
