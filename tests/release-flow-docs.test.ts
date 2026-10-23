import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

// P2-32 — mono 에는 plugin tag push 로 도는 워크플로가 없어요. 공개 레포 미러와 GitHub release 는
// `repo · mirror(public)` → `plugin · release` 를 손으로 실행해야 나가요. 릴리즈 안내가 "tag push 로
// release.yml 이 돈다" 고 말하면 미러를 빼먹고, 공개 레포 main 을 읽는 사용자는 옛 버전에 머물러요.
const REPO_ROOT = join(import.meta.dir, "..");
const read = (path: string): string => readFileSync(join(REPO_ROOT, path), "utf8");

const versionrc = JSON.parse(read(".versionrc.json")) as {
  tagPrefix: string;
  scripts: { postchangelog: string };
};
const GUIDES: Record<string, string> = {
  ".versionrc.json postchangelog": versionrc.scripts.postchangelog,
  "scripts/release-tag.ts": read("scripts/release-tag.ts"),
  "docs/policy/dev-policy.md": read("docs/policy/dev-policy.md"),
  "CLAUDE.md": read("CLAUDE.md"),
};
const MIRROR_COMMAND = "gh workflow run repo-mirror.yml --ref";
const RELEASE_COMMAND = "gh workflow run plugin-release.yml -f tag=";

describe("release flow guidance (P2-32)", () => {
  test("tag push 가 릴리스 워크플로를 띄운다고 안내하지 않아요", () => {
    for (const [name, text] of Object.entries(GUIDES)) {
      expect(text, name).not.toMatch(/release\.yml[^\n]*fire/);
    }
  });

  test("tag 뒤에 미러(dry-run 먼저) → plugin · release 순서를 안내해요", () => {
    for (const [name, text] of Object.entries(GUIDES)) {
      const mirror = text.indexOf(MIRROR_COMMAND);
      expect(mirror, `${name}: 미러 명령 없음`).toBeGreaterThanOrEqual(0);
      expect(text.indexOf(RELEASE_COMMAND), `${name}: plugin · release 가 미러 뒤에 없음`).toBeGreaterThan(mirror);
      expect(text, `${name}: target=plugin 없음`).toContain("-f target=plugin");
      expect(text, `${name}: dry-run 먼저 안 함`).toContain("-f dry_run=true");
      expect(text, `${name}: 실제 미러 실행 없음`).toContain("dry_run=false");
    }
  });

  test("정책 문서의 tag 형식이 .versionrc.json tagPrefix 와 같아요", () => {
    expect(versionrc.tagPrefix).toBe("plugin-v");
    for (const name of ["docs/policy/dev-policy.md", "CLAUDE.md"]) {
      expect(GUIDES[name], name).toContain(`--ref ${versionrc.tagPrefix}X.Y.Z`);
      expect(GUIDES[name], name).toContain(`tag=${versionrc.tagPrefix}X.Y.Z`);
    }
  });
});
