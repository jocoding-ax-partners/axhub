import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

// WP-I1 — skill 본문이 실제 CLI(clients/cli/axhub/src)·host 동작과 어긋난 지점의 회귀 잠금이에요.
// 각 test 이름의 ID 는 plan 부록 A 의 결함 ID 예요.
const REPO_ROOT = join(import.meta.dir, "..");
const read = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");

const VERIFY_WAIT = "--wait --wait-interval 20s --wait-timeout 9m --json";
const VERIFY_WAIT_SKILLS = ["skills/deploy/SKILL.md", "skills/import/SKILL.md", "skills/up/SKILL.md", "skills/deploy/references/workflow-details.md"];

describe("P1-13·DX26 deploy verify --wait 는 host 도구 timeout 안에서 끝나요", () => {
  // Claude Code Bash 기본 timeout 은 2분, 최대 600000ms 예요. 10분 CLI 대기는 도구가 먼저 끊어
  // verify 가 exit 0/6/7 을 돌려주지 못했어요.
  test("CLI 대기는 9분이고 명령 도구 timeout 600000ms 지시가 함께 있어요", () => {
    for (const path of VERIFY_WAIT_SKILLS) {
      const body = read(path);
      expect(body, path).toContain(VERIFY_WAIT);
      expect(body, path).not.toContain("--wait-timeout 10m");
      expect(body, path).toContain("600000ms");
    }
  });

  test("exit 6·10·도구 timeout 끊김은 실패가 아니라 재개 요약과 같은 verify 명령 한 줄이에요", () => {
    const deploy = read("skills/deploy/SKILL.md");
    expect(deploy).toContain("`6`·`10`·도구 timeout 끊김");
    expect(deploy).toContain("재개 요약과 같은 verify `--wait` 명령 한 줄");
    expect(read("skills/up/SKILL.md")).toContain("| 6·10·도구 timeout 끊김 |");
    expect(read("skills/import/SKILL.md")).toContain("exit 6·10 이거나 도구 timeout 으로 끊기면");
    expect(read("skills/deploy/references/workflow-details.md")).toContain("6, 10(CLI timeout), or the command tool cutting the call");
    const policy = read("docs/policy/agent-policy.md");
    expect(policy).toContain('"--wait --wait-interval 20s --wait-timeout 9m", "600000ms"');
  });
});

describe("P2-34·P3-48·P3-49 update 스킬은 CLI check 출력을 실제 의미대로 읽어요", () => {
  const SKILLS = ["skills/update/SKILL.md", "codex-overrides/skills/update/SKILL.md"];

  test("P2-34 disabled 는 업데이트 확인 꺼짐이고 brew·패키지 매니저 안내가 없어요", () => {
    for (const path of SKILLS) {
      const body = read(path);
      expect(body, path).toContain("AXHUB_DISABLE_AUTOUPDATE");
      expect(body, path).toContain("~/.axhub/config/disable-autoupdate");
      expect(body, path).not.toContain("brew upgrade");
      expect(body, path).not.toContain("패키지 매니저가 관리하는 설치");
    }
  });

  test("P2-34 feed_error·빈 latest 는 이미 최신이 아니라 확인 실패예요", () => {
    for (const path of SKILLS) {
      const body = read(path);
      expect(body, path).toContain("`feed_error` 가 있거나 `disabled` 가 아닌데 `latest` 가 비면");
      expect(body, path).toContain("`has_update == false` 를 최신으로 읽지 않고");
      expect(body, path).toContain("NEVER `feed_error`·빈 `latest` 를 \"이미 최신\" 으로 말하지 말아요");
    }
  });

  test("P3-48 AXHUB_NO_AUTO_UPDATE 는 백그라운드 훅만 꺼요 — POLICY.md 의 '끈 상태에서도 update 스킬로 업데이트' 와 일치해요", () => {
    expect(read("POLICY.md")).toContain("끈 상태에서 업데이트하려면");
    for (const path of [...SKILLS, "skills/update/references/plugin-update.md", "codex-overrides/skills/update/references/plugin-update.md"]) {
      const body = read(path);
      expect(body, path).toContain("백그라운드 자동 업데이트 훅만 꺼요");
      expect(body, path).not.toContain("AXHUB_NO_AUTO_UPDATE 설정이라 자동 적용은 안 해요");
      expect(body, path).not.toContain("업데이트 보류(AXHUB_NO_AUTO_UPDATE)");
    }
  });

  test("P3-49 '항상 즉시 확인' 대신 --json 의 1시간 캐시를 밝혀요", () => {
    for (const path of SKILLS) {
      const body = read(path);
      expect(body, path).not.toContain("항상 즉시 확인");
      expect(body, path).toContain("feed 를 최대 1시간 캐시해서");
    }
  });
});

describe("P2-35 배포 전 맞추기는 .env.local 에만 있는 이름도 등록해요", () => {
  test("deploy·up 은 .env.local 이름을 --from-dotenv .env.local 로 등록해요", () => {
    for (const path of ["skills/deploy/SKILL.md", "skills/up/SKILL.md"]) {
      const body = read(path);
      expect(body, path).toContain("`--from-dotenv .env.local`");
      expect(body, path).toContain("--from-dotenv .env --only <이름,…> --execute");
    }
  });
});

describe("P2-36 staging 배포 진단은 staging 로그를 읽어요", () => {
  test("logs 호출은 target_environment 가 staging 이면 --environment staging 을 붙여요", () => {
    for (const path of ["skills/diagnosis/SKILL.md", "skills/deploy/SKILL.md", "skills/deploy/references/workflow-details.md"]) {
      const body = read(path);
      expect(body, path).toContain("`target_environment`");
      expect(body, path).toContain("`--environment staging`");
    }
  });
});

describe("P2-37 clarity 는 hidden 이지만 지원되는 grant 생성 leaf 를 직접 확정해요", () => {
  test("authz grants create 를 이름으로 확정하고 set-scope 로 대신하지 않아요", () => {
    for (const path of ["skills/clarity/SKILL.md", "skills/clarity/references/execution-guardrails.md"]) {
      const body = read(path);
      expect(body, path).toContain("axhub authz grants create --help");
      expect(body, path).toContain("`set-scope`");
    }
    expect(read("skills/clarity/SKILL.md")).toContain("(hidden `authz grants create` 는 예외)");
  });
});

describe("P2-38 development 는 raw Postgres 테이블로 존재를 확인해요", () => {
  test("존재 확인은 raw_table_list·tables db-list 가 먼저이고 table_list 는 legacy managed 앱 전용이에요", () => {
    const gate = read("skills/development/references/write-gate.md");
    expect(gate).toContain("`raw_table_list({app_id})`");
    expect(gate).toContain("axhub tables db-list --app <app> --json");
    expect(gate).not.toContain("1. **존재 우선**: 정확한 `app_id`로 `table_list({app_id})`를 한 번 호출해");
    expect(read("skills/development/references/sdk-db-surface.md")).toContain("코드가 참조하는데 `raw_table_list`");
    expect(read("skills/development/SKILL.md")).toContain("`raw_table_list({app_id})`·`raw_row_list` 가 먼저예요");
  });
});

describe("P2-44·P2-45·P3-42 up 의 재실행·진행 중 배포·id 출처", () => {
  const up = read("skills/up/SKILL.md");

  test("P2-44 app_ambiguous 뒤 deploy-prep 을 --app 으로 다시 돌리지 않아요", () => {
    expect(up).toContain("`resolve.matched_apps`");
    expect(up).not.toContain("`--app` 에 그 값을 넣어 1단계를 다시 해요");
  });

  test("P2-45 진행 중 배포는 exit 9 already_exists 로 오고 in-flight id 로 verify 해요", () => {
    expect(up).toContain("exit 9 `already_exists`");
    expect(up).toContain("axhub plugin-support deploy-prep --refresh-in-flight --json");
    const deploy = read("skills/deploy/SKILL.md");
    expect(deploy).toContain("exit 9 + `already_exists`");
    expect(deploy).toContain("NEVER retry `axhub deploy create` on exit 9 `already_exists` or exit 64.");
    expect(read("skills/deploy/references/workflow-details.md")).toContain("On exit 9 with `already_exists`");
  });

  test("P3-42 DEPLOY_ID 출처 규칙이 하나예요 — deploy list 로 고르지 않아요", () => {
    expect(up).toContain("셋에서만 바인딩해요");
    expect(up).not.toContain("`deploy list --app \"$APP_ID\"` 로 방금 만들어진 배포가 있는지");
  });
});

describe("P3-43 import 배포 방식 카드는 manifest 가 선언한 방식이면 묻지 않아요", () => {
  test("axhub.yaml 이 deploy_method 를 선언하면 --deploy-method 로 다시 받지 않아요", () => {
    expect(read("skills/import/SKILL.md")).toContain("`build.deploy_method`·`deploy.method` 를 선언하지 않았으면");
  });
});

describe("P3-45·P3-46·P3-47 deploy reference drift", () => {
  const details = read("skills/deploy/references/workflow-details.md");

  test("P3-45 create field-expr 는 SKILL.md 와 같은 envelope-safe 식이에요", () => {
    expect(details).not.toContain("--field-expr '.id // .deployment_id // empty'");
    expect(details).toContain("--execute --field-expr '.data.id // .data.deployment_id // .id // .deployment_id // empty'");
  });

  test("P3-46 deploy list 에 없는 created_at 으로 고르지 않아요", () => {
    expect(details).not.toContain("choose the newest `created_at`");
    expect(read("skills/deploy/SKILL.md")).not.toContain("`created_at`이 가장 최신");
  });

  test("P3-47 session-carryover 는 없는 deploy 단계를 가리키지 않아요", () => {
    const carryover = read("skills/deploy/references/session-carryover.md");
    expect(carryover).not.toContain("infer-tables-env) AskUserQuestion");
    expect(carryover).toContain("`## 배포 전 맞추기`(AP-29)는 그대로 해요");
    expect(read("skills/development/references/sdk-db-surface.md")).not.toContain("사전 점검 질문이 **중복되지 않게**");
  });
});

describe("P3-37 auto-update 문서는 실제 재시도 주기를 말해요", () => {
  test("headless 회차 유실은 lock TTL 이 아니라 24h 뒤 재시도예요", () => {
    const doc = read("CLAUDE.md");
    expect(doc).not.toContain("그 회차는 잃고 lock TTL 뒤 재시도해요");
    expect(doc).toContain("24h 뒤 다음 주기에 재시도해요");
    // 근거: worker 는 lock 직후 throttle 캐시를 touch 해요.
    const hook = read("hooks/session-auto-update.sh");
    expect(hook.indexOf(': > "$CACHE"')).toBeGreaterThan(hook.indexOf('mkdir "$LOCK"'));
    expect(hook.indexOf(': > "$CACHE"')).toBeLessThan(hook.indexOf("update check"));
  });
});
