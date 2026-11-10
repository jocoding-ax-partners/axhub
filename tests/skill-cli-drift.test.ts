import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

// WP-I1 — skill 본문이 실제 CLI(clients/cli/axhub/src)·host 동작과 어긋난 지점의 회귀 잠금이에요.
// 각 test 이름의 ID 는 plan 부록 A 의 결함 ID 예요.
const REPO_ROOT = join(import.meta.dir, "..");
const read = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");

describe("P1-13·DX26 deploy verify --wait 는 host 도구 timeout 안에서 끝나요", () => {
  // Claude Code Bash 기본 timeout 은 2분, 최대 600000ms 예요. 10분 CLI 대기는 도구가 먼저 끊어
  // verify 가 exit 0/6/7 을 돌려주지 못했어요. 2026-11 deploy 개편판은 import 처럼 `--wait-timeout 9m` 과
  // "명령 시간 제한 10분" 으로 적어요(옛 `--wait-interval 20s`·`600000ms` 문구·재개 요약 표는 뺐어요).
  test("deploy 는 push 대기 한 번 뒤 끊기면 다시 올리지 않고 9분 verify 로 이어가요", () => {
    const deploy = read("skills/deploy/SKILL.md");
    expect(deploy).toContain("`--execute --wait` 를 붙여 한 번만 실행해요(명령 시간 제한 10분)");
    expect(deploy).toContain("--wait --wait-timeout 9m --json");
    expect(deploy).not.toContain("--wait-timeout 10m");
    expect(deploy).toContain("기다리다 끊기면 실패가 아니에요. 다시 올리지 말고");
    const policy = read("docs/policy/agent-policy.md");
    expect(policy).toContain('"--wait --wait-timeout 9m --json", "명령 시간 제한 10분"');
  });

  // import·bootstrap 은 bootstrap 개편 형식대로 `--wait --wait-timeout 9m` 한 번과 "명령 시간 제한 10분" 으로 적어요.
  test("import 도 9분 대기 한 번이고 시간 안에 안 끝난 건 실패가 아니에요", () => {
    const body = read("skills/import/SKILL.md");
    expect(body).toContain("--wait --wait-timeout 9m --json");
    expect(body).toContain("(명령 시간 제한 10분)");
    expect(body).not.toContain("--wait-timeout 10m");
    expect(body).toContain("시간 안에 안 끝난 건(exit 6·10) 실패가 아니에요");
    expect(body).toContain("기다리다 끊기면 다시 실행하지 말고 같은 확인 명령으로 이어가요");
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

// P2-35(.env.local 등록)·P2-36(staging 로그 --environment)·P2-44·P2-45·P3-42(up 의 재실행·진행 중 배포·id 출처)·
// P3-45·P3-46·P3-47(deploy reference) 는 2026-11 deploy 개편 때 옛 deploy 본문·up·diagnosis·references 와 함께
// 뺐어요. 배포 번호 찾기·진행 중 배포 대기는 CLI `deploy push` 가 맡아요.

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
