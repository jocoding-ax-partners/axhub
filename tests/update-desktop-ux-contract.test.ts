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
    expect(update).toContain("출력이 길어도 전체 `claude plugin list` 결과를 도구 응답에서 내부적으로 읽고");
    expect(update).toContain("파일 읽기 도구를 쓰지 않아요");
    expect(update).toContain("`claude` CLI 가 없거나 목록에서 못 찾으면 `<PLUGIN_VERSION>` 없이 CLI 업데이트 확인만 진행해요");
    expect(update).toContain("scope 원문, 영어 진행 로그는 사용자에게 말하지 않아요");
  });

  test("does not continue mixed app-status requests inside update", () => {
    const update = readRepo("skills/update/SKILL.md");

    expect(update).toContain("버전 확인과 다른 axhub 운영 요청을 함께 말해도");
    expect(update).toContain("버전 확인/업데이트 결과까지만");
    expect(update).toContain("앱 상태는 이어서 확인할게요.");
    expect(update).toContain("`App list (axhub)`, `Tenant recent deployments (axhub)`, `App get (axhub)`");
    expect(update).toContain("read 작업이어도 `clarity` 소관이에요");
  });
});
