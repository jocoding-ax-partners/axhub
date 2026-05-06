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
    // git-init nested checklist 만 허용 — main stage markdown 다시 들어오면 count > 1 로 fail.
    expect((content.match(/작업 단계/g) || []).length).toBe(1);
    expect(content).toContain("□ git 저장소 만들기");
    expect(content).toContain("□ 파일을 첫 저장 지점에 담기");
    expect(content).toContain("□ 배포 정보 다시 확인하기");
    expect(content).toContain("git init");
    expect(content).toContain("git branch -M main");
    expect(content).toContain("git add -A");
    expect(content).toContain('git commit -m "init: axhub deploy baseline"');
    expect(content).toContain("resolve --intent deploy");
  });

  test("replaces stale TodoWrite state when entering deploy git readiness", () => {
    const content = readFileSync(DEPLOY_SKILL, "utf8");

    expect(content).toContain("위 배열 전체로 교체해요");
    expect(content).toContain("이전 스킬 todo 가 화면에 남아 있으면 Step 1 전에 deploy 목록만 보이도록 다시 호출해요");
    expect(content).toContain("replace the full TodoWrite list with the local git readiness checklist");
    expect(content).toContain('content: "git 저장소 만들기"');
    expect(content).toContain("이 TodoWrite 호출도 기존 목록을 기준으로 patch 하지 말고 전체 교체로 실행해요");
  });

  test("non-interactive fallback never initializes git automatically", () => {
    const registry = JSON.parse(readFileSync(ASK_DEFAULTS, "utf8"));
    const entry = registry.deploy["배포 전 저장 지점을 만들까요?"];

    expect(entry.safe_default).toBe("명령어만 보기");
    expect(entry.rationale).toContain("non-interactive");
    expect(entry.rationale).toContain("git init");
  });

  test("git-init and deploy preview questions are structured AskUserQuestion payloads", () => {
    const content = readFileSync(DEPLOY_SKILL, "utf8");
    const registry = JSON.parse(readFileSync(ASK_DEFAULTS, "utf8"));

    expect(content).toContain('"question": "배포 전 저장 지점을 만들까요?"');
    expect(content).toContain('"header": "저장 지점"');
    expect(content).toContain('"value": "init_and_continue"');
    expect(content).toContain('"value": "show_commands"');
    expect(content).toContain('"value": "abort"');

    expect(content).toContain("Then ask with structured AskUserQuestion JSON");
    expect(content).toContain('"question": "진행할까요?"');
    expect(content).toContain('"header": "배포 확인"');
    expect(content).toContain('"label": "네, 배포"');
    expect(content).toContain('"value": "approve"');
    expect(content).toContain('"label": "미리보기만"');
    expect(content).toContain('"value": "dry_run"');
    expect(content).toContain("If the user chooses `dry_run`, add `--dry-run`");

    expect(registry.deploy["진행할까요?"].safe_default).toBe("미리보기만");
    expect(registry.deploy["진행할까요?"].rationale).toContain("dry-run");
  });

  test("uses bootstrap plan/record before destructive deploy commands", () => {
    const content = readFileSync(DEPLOY_SKILL, "utf8");

    expect(content.indexOf("Live resolve first")).toBeLessThan(
      content.indexOf("First-run bootstrap plan/record bridge"),
    );
    expect(content).toContain("do **not** run `bootstrap apps_create`");
    expect(content).toContain("do not mint or run a second `deploy_create`");
    expect(content).toContain("axhub-helpers bootstrap --auto-chain --json");
    expect(content).toContain("bootstrap --record");
    expect(content).toContain("pending_action_id");
    expect(content).toContain("pending_action_hash");
    expect(content).toContain("binding_hash");
    expect(content).toContain("no_retry_without_confirmed_idempotency");
    expect(content).toContain("schema_version");
    expect(content).toContain("bootstrap-record/v1");
  });

  test("github connection blocker shows a direct link instead of slash-command handoff", () => {
    const content = readFileSync(DEPLOY_SKILL, "utf8");

    expect(content).toContain("github.git_connection_required");
    expect(content).toContain("GitHub 저장소 연결");
    expect(content).toContain('axhub github repos list --json');
    expect(content).toContain("GitHub 연결 링크: <install_url>");
    expect(content).toContain("https://github.com/new?name=$APP_SLUG");
    expect(content).toContain('axhub github connect "$APP_ID" --repo "$OWNER_REPO" --branch "$BRANCH" --account "$ACCOUNT" --json');
    expect(content).not.toContain("(/axhub:github 호출)");
  });

});
