import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const readRepo = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");

// 회귀: device code 를 한 번 놓치면 `github link` 재실행이 저장된 pending link 를
// 그대로 이어 줘서 이미 죽은 코드가 돌아오고, 사용자가 승인해도 연동이 안 풀렸어요.
// CLI 는 `--fresh`(saved pending 폐기 + 새 코드 발급)로 풀었는데 plugin 계약에는
// 반영이 없었어요. 아래 4 lane 전부에 재발급 경로가 살아 있어야 해요.
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

  test("bootstrap AP-18 fast path reissues instead of replaying a dead code", () => {
    const bootstrap = readRepo("skills/bootstrap/SKILL.md");

    expect(bootstrap).toContain("`github link --fresh`");
    expect(bootstrap).toContain("저장된 pending link 는 죽은 코드를 그대로 돌려줘요");
    expect(bootstrap).toContain("유효한 코드가 화면에 있으면 붙이지 않아요");
    // 구 CLI(exit 64) fallback 은 본문 byte 캡 때문에 reference 가 소유해요
    const errors = readRepo("skills/bootstrap/references/errors-and-followups.md");
    expect(errors).toContain("`--fresh` 가 exit 64 로 거부되면 그 플래그를 모르는 구 CLI 라");
  });

  test("post-update reconnect lane allows the fresh variant as a start command", () => {
    for (const path of [
      "skills/update/references/post-update-continuation.md",
      "codex-overrides/skills/update/references/post-update-continuation.md",
    ]) {
      const doc = readRepo(path);
      // "정확히 아래 계열만 써요" allowlist 에 실제로 들어 있어야 해요
      expect(doc, path).toContain(`\n${FRESH_COMMAND}\n`);
      expect(doc, path).toContain("저장된 pending link 가 이어져요(새 코드 발급 없음)");
      expect(doc, path).toContain("승인 자체를 못 했으면 그 저장된 pending link 가 죽은 코드를 그대로 돌려줘요");
    }
  });

  test("update routing hooks permit the fresh variant only for a lost or expired code", () => {
    for (const path of ["hooks/update-router.sh", "hooks/session-update-router-guard.sh"]) {
      const hook = readRepo(path);
      expect(hook, path).toContain("AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub --no-input github link --fresh");
      expect(hook, path).toContain("when the earlier device code was lost or expired");
      expect(hook, path).toContain("never while a valid code is still on screen");
    }
    for (const path of [
      "codex-overrides/hooks/context/update-router.md",
      "codex-overrides/hooks/context/update-first.md",
    ]) {
      expect(readRepo(path), path).toContain("앞선 코드를 놓쳤거나 만료됐을 때만");
    }
  });

  test("github_relogin_required lanes point at the reissue instead of a bare retry", () => {
    for (const path of [
      "skills/deploy/references/error-empathy-catalog.md",
      "skills/bootstrap/references/errors-and-followups.md",
      "skills/bootstrap/references/templates-and-github.md",
      "skills/onboarding/references/github-app.md",
      "skills/scaffold/SKILL.md",
      "skills/import/SKILL.md",
    ]) {
      expect(readRepo(path), path).toContain("--fresh");
    }
  });
});
