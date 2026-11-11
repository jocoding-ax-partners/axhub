import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");

// 회귀: device code 를 한 번 놓치면 `github link` 재실행이 저장된 pending link 를
// 그대로 이어 줘서 이미 죽은 코드가 돌아오고, 사용자가 승인해도 연동이 안 풀렸어요.
// CLI 는 `--fresh`(saved pending 폐기 + 새 코드 발급)로 풀었는데 plugin 계약에는
// 반영이 없었어요. 아래 lane 전부에 재발급 경로가 살아 있어야 해요.

describe("github link --fresh reissue contract", () => {
  // GitHub 다시 연결은 2026-11 운영 스킬 개편(clarity → operate) 뒤에도 operate 가 맡아요 — 첫 연결은 bootstrap 4번 (AP-18).
  test("operate reconnect lane shows the code, waits once and reissues an expired code", () => {
    const operate = readRepo("skills/operate/SKILL.md");

    expect(operate).toContain("GitHub 다시 연결 `axhub github link`");
    expect(operate).toContain("출력의 주소와 입력 코드를 채팅 본문에 바로 보여주고");
    expect(operate).toContain("승인은 한 번만(`--wait-approval 90s`) 기다려요");
    expect(operate).toContain("코드가 만료됐으면 `--fresh` 로 새로 받아요");
    // setup 은 이 컴퓨터 준비(CLI·로그인·git)만 보고 GitHub 연결을 하지 않아요.
    expect(readRepo("skills/setup/SKILL.md")).not.toContain("github link");
  });


  test("bootstrap reissues an expired code with --fresh instead of replaying it", () => {
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");

    // bootstrap 은 saga 밖에서 `github link --json` 으로 코드를 먼저 받고, 만료면 `--fresh` 로 새로 받아요.
    expect(bootstrap).toContain("axhub github link --json");
    expect(bootstrap).toContain("코드가 만료됐으면 `--fresh` 로 새로 받아요");
  });

  test("github_relogin_required lanes point at the reissue instead of a bare retry", () => {
    // deploy 는 2026-11 개편 때 GitHub 재연결 안내를 빼고 CLI 의 `recovery_action`·`next` 를 따라요.
    expect(readRepo("skills/bootstrap/SKILL.md")).toContain("--fresh");
    // import 는 GitHub 연결을 bootstrap 3·4단계와 똑같이 해요 — 코드 재발급은 그 절차를 따라요.
    expect(readRepo("skills/import/SKILL.md")).toContain("GitHub 연결은 bootstrap 스킬 3·4단계와 똑같이 해요");
  });
});
