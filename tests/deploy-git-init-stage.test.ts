import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const DEPLOY_SKILL = join(REPO_ROOT, "skills/deploy/SKILL.md");
const ASK_DEFAULTS = join(REPO_ROOT, "tests/fixtures/ask-defaults/registry.json");

describe("deploy skill git init stage", () => {
  test("offers a user-approved git init + first commit recovery before deploy", () => {
    const content = readFileSync(DEPLOY_SKILL, "utf8");

    expect(content).toContain("git_init_needed");
    expect(content).toContain("배포 전 저장 지점을 만들까요?");
    expect(content).toContain("작업 단계");
    expect(content).toContain("□ git 저장소 만들기");
    expect(content).toContain("□ 파일을 첫 저장 지점에 담기");
    expect(content).toContain("□ 배포 정보 다시 확인하기");
    expect(content).toContain("git init");
    expect(content).toContain("git branch -M main");
    expect(content).toContain("git add -A");
    expect(content).toContain('git commit -m "init: axhub deploy baseline"');
    expect(content).toContain("resolve --intent deploy");
  });

  test("non-interactive fallback never initializes git automatically", () => {
    const registry = JSON.parse(readFileSync(ASK_DEFAULTS, "utf8"));
    const entry = registry.deploy["배포 전 저장 지점을 만들까요?"];

    expect(entry.safe_default).toBe("명령어만 보기");
    expect(entry.rationale).toContain("non-interactive");
    expect(entry.rationale).toContain("git init");
  });
});
