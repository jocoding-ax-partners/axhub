import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");

describe("update Desktop UX contract", () => {
  test("plugin manifest preserves update no-preamble routing contract", () => {
    const manifest = JSON.parse(readRepo(".claude-plugin/plugin.json")) as { description: string };

    expect(manifest.description).toContain("update 는 사전 안내 문장 없이 `현재 버전을 확인할게요.` 로 바로 시작해요");
  });

  test("does not read plugin cache manifests in Claude Desktop", () => {
    const update = readRepo("skills/update/SKILL.md");

    expect(update).toContain("Invoke this skill before writing any explanatory assistant sentence.");
    expect(update).toContain("스킬 실행 전 사용자 문장 0개");
    expect(update).toContain("Start directly; do not explain why this path was chosen or name the chosen skill.");
    expect(update).toContain("Mixed app-status/log/deploy requests after update still stop after version check/update");
    expect(update).toContain('write exactly "업데이트 확인은 끝났어요. 앱 상태 확인은 별도 상태 확인 흐름에서 처리해요." and end the assistant turn');
    expect(update).toContain("Claude Desktop 에서는 `${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json` 같은 플러그인 캐시 파일을 읽지 않아요");
    expect(update).toContain("스킬 호출 전 사전 안내 문장도 쓰지 않아요");
    expect(update).toContain("작업 디렉토리 밖이라 초보자에게 불필요한 읽기 권한 팝업이 떠요");
    expect(update).toContain("정확히 `claude plugin list` 만 실행한 출력");
    expect(update).toContain("`claude plugin list 2>&1`");
    expect(update).toContain("`claude plugin list 2>&1 | grep ...`");
    expect(update).toContain("redirect");
    expect(update).toContain("실행하려는 command 가 `claude plugin list 2>&1` 로 떠오르면 **반드시 `claude plugin list` 로 바꿔요.**");
    expect(update).toContain("플러그인 버전, 설치 scope, 다음 CLI 확인을 영어 내부 로그처럼 chat 에 쓰지 말고");
    expect(update).toContain("필요한 경우 `현재 플러그인 버전을 확인했어요.` 라고만 말해요");
    expect(update).toContain("버전과 설치 위치를 같은 문장에 섞지 않아요");
    expect(update).toContain("설치 위치값은 업데이트 명령의 `--scope` 인자에만 쓰고 chat 에는 쓰지 않아요");
    expect(update).toContain("플러그인 확인 직후에는 `현재 플러그인 버전을 확인했어요.` 만 보여줘요");
    expect(update).toContain("출력이 길어도 전체 `claude plugin list` 결과를 도구 응답에서 내부적으로 읽고");
    expect(update).toContain("파일 읽기 도구를 쓰지 않아요");
    expect(update).toContain("`claude` CLI 가 없거나 목록에서 못 찾으면 `<PLUGIN_VERSION>` 없이 CLI 업데이트 확인만 진행해요");
    expect(update).toContain("scope 원문, 영어 진행 로그는 사용자에게 말하지 않아요");
  });

  test("uses the newest enabled plugin entry when Claude lists duplicates", () => {
    const update = readRepo("skills/update/SKILL.md");

    expect(update).toContain("`claude plugin list` 에 `axhub@axhub` 가 여러 번 나오면");
    expect(update).toContain("enabled 항목 중 **가장 높은 semver** 를 `<PLUGIN_VERSION>` 으로 삼아요");
    expect(update).toContain("`local` → `project` → `user`");
    expect(update).toContain("낮은 버전이 함께 남아 있어도 사용자에게 중복 설치·scope 원문을 설명하지 않고");
    expect(update).toContain("성공하면 `claude plugin list` 를 한 번 더 실행해");
    expect(update).toContain("이 값이 `plugin.latest` 보다 높으면 `<PLUGIN_UPDATED_VERSION>` 을 최종 카드에 써요");
    expect(update).toContain("낡은 중복 항목을 나열하지 않아요");
    expect(update).toContain("NEVER `claude plugin list` 에서 처음 발견한 낡은 `axhub@axhub` 항목만 보고 업데이트 여부를 판단하지 말아요");
  });

  test("does not continue mixed app-status requests inside update", () => {
    const update = readRepo("skills/update/SKILL.md");

    expect(update).toContain("버전 확인과 다른 axhub 운영 요청을 함께 말해도");
    expect(update).toContain("버전 확인/업데이트 결과까지만");
    expect(update).toContain("업데이트 확인은 끝났어요. 앱 상태 확인은 별도 상태 확인 흐름에서 처리해요.");
    expect(update).toContain("`App list (axhub)`, `Tenant recent deployments (axhub)`, `App get (axhub)`");
    expect(update).toContain("read 작업이어도 이 스킬에서는 금지예요");
    expect(update).toContain("원문이 영어로 `then`, `and then`, `after that`, `help me understand` 를 써도");
    expect(update).toContain("현재 assistant turn 을 즉시 끝내요");
    expect(update).toContain("update 실행 뒤 같은 assistant 응답 안에서 앱 목록·배포 상태 도구, 추가 Bash/MCP/App 도구");
    expect(update).toContain("`앱 상태 조회`, `배포 상태 조회`, `최근 배포 조회` 같은 tool 제목이 떠올랐다면");
    expect(update).toContain("Task/Subagent/Agent/백그라운드 작업을 절대 만들지 않아요");
    expect(update).toContain("NEVER Task/Subagent/Agent/백그라운드 작업으로 mixed request 의 남은 앱 상태 확인을 우회하지 말아요");
    expect(update).toContain("후속 작업을 약속하거나 실제 상태를 요약하는 문장은 쓰지 않아요");
    expect(update).toContain("이제 이어서 앱 상태를 확인할게요");
    expect(update).toContain("현재 앱 상태를 정리해드릴게요");
    expect(update).not.toContain("다음 동작은 `clarity`");
    expect(update).not.toContain("그 다음 작업은 `clarity`");
  });
});
