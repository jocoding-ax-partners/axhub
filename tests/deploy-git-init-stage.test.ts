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
    expect(content).toContain("Do not render this plan as a markdown checklist");
    expect(content).not.toContain("작업 단계");
    expect(content).not.toMatch(/^\s*(?:└\s*)?□\s/m);
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

    expect(entry.safe_default).toBe("취소");
    expect(entry.rationale).toContain("non-interactive");
    expect(entry.rationale).toContain("git init");
    expect(entry.allowed_safe_defaults).toContain("취소");
    expect(entry.allowed_safe_defaults).not.toContain("명령어만 보기");
  });

  test("auto-resolves CLAUDE_PLUGIN_ROOT before helper preflight and deploy commands", () => {
    const content = readFileSync(DEPLOY_SKILL, "utf8");

    expect(content).toContain("CLAUDE_PLUGIN_ROOT 자동 확인");
    expect(content).toContain("CLAUDE_SKILL_DIR");
    expect(content).toContain("export CLAUDE_PLUGIN_ROOT");
    expect(content).toContain('PATH="${CLAUDE_PLUGIN_ROOT}/bin:${PATH}"');
  });

  test("pre-execute helper root resolution covers native Windows, not bash-only (PowerShell lane)", () => {
    const content = readFileSync(DEPLOY_SKILL, "utf8");

    // Phase 27 (ADR-0013): the load-time node-runner `!command` injection is retired.
    // Deploy's cross-platform root resolution now lives in the PowerShell setup prose
    // (native Windows .exe lane) alongside the POSIX bash setup — not a node runner.
    expect(content).toContain("Windows PowerShell");
    expect(content).toContain("axhub-helpers.exe");
    expect(content).toContain('$env:CLAUDE_PLUGIN_ROOT');
    // No leftover load-time injection; preflight runs in-body instead.
    expect(content).not.toMatch(/^!`node -e/m);
    expect(content).toMatch(/preflight\s+--json/);
  });

  test("git-init and deploy preview questions are structured AskUserQuestion payloads", () => {
    const content = readFileSync(DEPLOY_SKILL, "utf8");
    const registry = JSON.parse(readFileSync(ASK_DEFAULTS, "utf8"));

    expect(content).toContain('"question": "배포 전 저장 지점을 만들까요?"');
    expect(content).toContain('"header": "저장 지점"');
    expect(content).toContain('"value": "init_and_continue"');
    expect(content).toContain('"value": "abort"');
    expect(content).not.toContain('"value": "show_commands"');
    expect(content).toContain('"label": "지금 만들기"');

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

  test("Vibe Coder Visibility Rules block masks raw helper jargon from user chat", () => {
    const content = readFileSync(DEPLOY_SKILL, "utf8");

    expect(content).toContain("## Vibe Coder Visibility Rules");
    expect(content).toContain("internal verification primitives");
    expect(content).toContain("do not echo their raw values to the user chat");
    expect(content).toContain("AXHUB_DEPLOY_VERBOSE=1");

    // internal primitives enum 명시 — 모든 항목이 본문 안에 등장해야 잠금됨
    const lockedFields = [
      "binding_hash",
      "pending_action_id",
      "pending_action_hash",
      "command_argv",
      "consent_binding",
      "synthesized_by_helper",
      "retry_policy",
      "idempotency_key",
      "exit_code",
      "next_action",
      "schema_version",
      "bootstrap_plan",
    ];
    for (const field of lockedFields) {
      expect(content, `Visibility Rules must enumerate '${field}'`).toContain(field);
    }
  });

  test("cli_too_new dismiss question drops raw '(cli_too_new)' jargon", () => {
    const content = readFileSync(DEPLOY_SKILL, "utf8");
    const registry = JSON.parse(readFileSync(ASK_DEFAULTS, "utf8"));

    expect(content).toContain('"question": "axhub CLI 가 더 최신 버전인데 계속할까요?"');
    expect(content).not.toContain('"question": "axhub CLI 새 버전 (cli_too_new) 인데 계속할까요?"');

    const entry = registry.deploy["axhub CLI 가 더 최신 버전인데 계속할까요?"];
    expect(entry).toBeDefined();
    expect(entry.safe_default).toBe("계속해요");
    expect(registry.deploy["axhub CLI 새 버전 (cli_too_new) 인데 계속할까요?"]).toBeUndefined();
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
    expect(content).toContain("precondition_failed");
    expect(content).toContain("GitHub 연결이 먼저 필요해요");
    expect(content).toContain('axhub apps git status --app "$APP_ID" --json');
    expect(content).toContain("GitHub 연결 링크: <install_url>");
    expect(content).toContain("https://github.com/new?name=$APP_SLUG");
    expect(content).toContain('axhub apps git connect --app "$APP_ID" --repo "$OWNER_REPO" --branch "$BRANCH" --execute --json');
    expect(content).not.toContain("(/axhub:github 호출)");
  });

  test("deploy precondition failures route subdomain setup before retrying deploy", () => {
    const content = readFileSync(DEPLOY_SKILL, "utf8");

    expect(content).toContain("subdomain_not_configured");
    expect(content).toContain("axhub apps update <slug> --subdomain <subdomain> --json");
    expect(content).toContain("subdomain 2..32자 제약");
    expect(content).toContain("apps_update");
    expect(content).toContain("deploy_create consent 를 새로 mint");
  });

  test("github connection blocker routes into guided github setup instead of ending on manual connect", () => {
    const content = readFileSync(DEPLOY_SKILL, "utf8");

    expect(content).toContain("route into `skills/github/SKILL.md` guided setup/connect");
    expect(content).toContain("do not end with a manual connect command as the next step");
    expect(content).toContain("GitHub guided setup/connect owns repo create, remote add, first push, and connect consent");
    expect(content).not.toContain("Then show the exact follow-up command without executing it until consent is minted in the github skill flow");
  });

  test("github setup AskUserQuestion fallbacks are conservative in subprocess", () => {
    const registry = JSON.parse(readFileSync(ASK_DEFAULTS, "utf8"));

    expect(registry.github["GitHub 연동 작업을 고를까요?"].safe_default).toBe("list_only");
    expect(registry.github["GitHub repo 를 만들까요?"].safe_default).toBe("abort");
    expect(registry.github["git remote 를 추가할까요?"].safe_default).toBe("abort");
    expect(registry.github["첫 push 를 실행할까요?"].safe_default).toBe("abort");
    expect(registry.github["axhub 앱에 repo 를 연결할까요?"].safe_default).toBe("abort");
  });

});
