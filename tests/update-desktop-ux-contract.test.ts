import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");

describe("update Desktop UX contract", () => {
  test("does not read plugin cache manifests in Claude Desktop", () => {
    const update = readRepo("skills/update/SKILL.md");

    expect(update).toContain("Claude Desktop 에서는 `${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json` 같은 플러그인 캐시 파일을 읽지 않아요");
    expect(update).toContain("작업 디렉토리 밖이라 초보자에게 불필요한 읽기 권한 팝업이 떠요");
    expect(update).toContain("정확히 `claude plugin list` 만 실행한 출력");
    expect(update).toContain("`claude plugin list 2>&1 | grep ...`");
    expect(update).toContain("출력이 길어도 전체 `claude plugin list` 결과를 도구 응답에서 내부적으로 읽고");
    expect(update).toContain("파일 읽기 도구를 쓰지 않아요");
    expect(update).toContain("`claude` CLI 가 없거나 목록에서 못 찾으면 `<PLUGIN_VERSION>` 없이 CLI 업데이트 확인만 진행해요");
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
