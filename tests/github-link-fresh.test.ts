import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");

// 회귀: device code 를 한 번 놓치면 `github link` 재실행이 저장된 pending link 를
// 그대로 이어 줘서 이미 죽은 코드가 돌아오고, 사용자가 승인해도 연동이 안 풀렸어요.
// CLI 는 `--fresh`(saved pending 폐기 + 새 코드 발급)로 풀었는데 plugin 계약에는
// 반영이 없었어요. 아래 lane 전부에 재발급 경로가 살아 있어야 해요.
const FRESH_COMMAND = "AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link --fresh";

describe("github link --fresh reissue contract", () => {
  test("clarity reconnect lane reissues a lost or expired code", () => {
    const clarity = readRepo("skills/clarity/SKILL.md");

    expect(clarity).toContain(FRESH_COMMAND);
    expect(clarity).toContain("저장된 pending device link 를 그대로 이어 줘서");
    // 유효한 코드를 보고 있는 사용자의 코드를 무효화하지 않는 반대 방향 가드
    expect(clarity).toContain("지금 보고 있는 코드가 아직 유효하면 `--fresh` 를 붙이지 않아요");
    expect(clarity).toContain("unknown flag(exit 64)");
    // 죽은 코드를 구 CLI 오진으로 흘려보내지 않아요
    expect(clarity).toContain("위 `--fresh` 재발급을 한 번 먼저 쓰고");
  });

  test("bootstrap reissues an expired code with --fresh instead of replaying it", () => {
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");

    // bootstrap 은 saga 밖에서 `github link --json` 으로 코드를 먼저 받고, 만료면 `--fresh` 로 새로 받아요.
    expect(bootstrap).toContain("axhub github link --json");
    expect(bootstrap).toContain("코드가 만료됐으면 `--fresh` 로 새로 받아요");
  });

  test("github_relogin_required lanes point at the reissue instead of a bare retry", () => {
    for (const path of [
      "skills/deploy/references/error-empathy-catalog.md",
      "skills/bootstrap/SKILL.md",
      "skills/import/SKILL.md",
    ]) {
      expect(readRepo(path), path).toContain("--fresh");
    }
  });
});
