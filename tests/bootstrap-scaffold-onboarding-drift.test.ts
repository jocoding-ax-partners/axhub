import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

// bootstrap·scaffold·onboarding skill 이 ax-hub-cli 실제 계약과 어긋났던 지점(WP-I1)을
// 문자열 계약으로 잠가요. 각 test 는 결함 ID 하나를 맡아요.

const ROOT = join(import.meta.dir, "..");
const read = (path: string): string => readFileSync(join(ROOT, path), "utf8");

const BOOTSTRAP = read("skills/bootstrap/SKILL.md");
const SCAFFOLD = read("skills/scaffold/SKILL.md");
const ONBOARDING = read("skills/onboarding/SKILL.md");
const LOCAL = read("skills/bootstrap/references/bootstrap-and-local.md");
const ERRORS = read("skills/bootstrap/references/errors-and-followups.md");
const BLOCKED = read("skills/bootstrap/references/github-blocked-local-deploy.md");
const RESUME = read("skills/bootstrap/references/resume-and-tenant.md");
const PLACEHOLDERS = read("skills/scaffold/references/placeholders.md");
const READY_CARD = read("skills/onboarding/references/ready-card.md");
const INSTALL = read("skills/onboarding/references/install-channels-and-auth.md");

const section = (doc: string, start: string, end: string): string => {
  const from = doc.indexOf(start);
  expect(from, `missing section start: ${start}`).toBeGreaterThanOrEqual(0);
  const to = doc.indexOf(end, from + start.length);
  expect(to, `missing section end: ${end}`).toBeGreaterThan(from);
  return doc.slice(from, to);
};

describe("bootstrap/scaffold/onboarding skill-CLI drift", () => {
  test("P2-39 scaffold placeholders follow the server: tenant slug, slug subdomain, AppHost origin", () => {
    const pick = section(SCAFFOLD, "### 3. 질문 한 번에", "### 4. 템플릿 내려받기");
    const substitution = section(SCAFFOLD, "### 5. placeholder 치환", "### 6. 커밋");

    // import 는 항상 `--subdomain <slug>` 로 앱을 만들어서 다른 subdomain 을 고르게 하지 않아요.
    expect(pick).toContain("subdomain 은 slug 그대로");
    expect(pick).not.toContain("다르게 원하면 조정");
    // tenant-resolve 의 `tenant` 는 UUID 일 수 있어서 `{{TENANT}}` 에는 slug 를 넣어요.
    expect(substitution).toContain("| `{{TENANT}}` | 작업공간 slug |");
    expect(substitution).not.toContain("3단계에서 정한 값");
    // 회사 작업공간은 서버 AppHost `{sub}.{slug}.{apex}`, 개인 작업공간은 서버가 주소를 만들어요.
    expect(substitution).toContain("회사 작업공간 `https://<slug>.<작업공간 slug>.axhub.ai` (서버 AppHost)");
    expect(substitution).toContain("8단계 import 뒤에 채워요");
    expect(substitution).not.toContain("`https://<subdomain>.<tenant>.axhub.ai`");
    expect(substitution).not.toContain("서버와 같은 조립식");
    expect(PLACEHOLDERS).toContain("같은 `tenant_id` 행의 `tenant_slug`");
    expect(PLACEHOLDERS).toContain("`tenant_kind` 가 `public` 이면 개인 작업공간");
    expect(PLACEHOLDERS).toContain("`axhub apps get <slug> --tenant <tenant> --json` 의 `subdomain`·`access_url`");
    expect(PLACEHOLDERS).toContain("import 는 subdomain 옵션 없이 항상 slug 를 subdomain 으로 써서");

    // bootstrap 12단계 fallback 도 앱을 만든 뒤 그 앱 값으로 주소 토큰을 채워요.
    const fallback = section(BOOTSTRAP, "### 12. GitHub 이 막혔을 때", "## NEVER");
    expect(fallback).toContain("`{{APP_SUBDOMAIN}}`·`{{APP_ORIGIN}}`→`apps get` 의 `subdomain`·`access_url`");
    expect(fallback).toContain("`{{TENANT}}`→작업공간 slug");
    expect(BLOCKED).toContain("| `{{APP_SUBDOMAIN}}` | `axhub apps get <slug> --tenant <tenant> --json` 의 `subdomain` |");
  });

  test("P2-40 {{API_BASE}} uses the API host the server substitutes", () => {
    for (const [name, doc] of [
      ["bootstrap", BOOTSTRAP],
      ["scaffold", SCAFFOLD],
      ["github-blocked-local-deploy", BLOCKED],
      ["scaffold placeholders", PLACEHOLDERS],
    ] as const) {
      expect(doc, name).toContain("prod `https://api.axhub.ai`");
      expect(doc, name).not.toContain("prod `https://axhub.ai`");
    }
  });

  test("P2-41 multi-workspace users pick from tenant-resolve --json candidates", () => {
    for (const [name, doc] of [
      ["bootstrap", BOOTSTRAP],
      ["scaffold", SCAFFOLD],
      ["resume-and-tenant", RESUME],
    ] as const) {
      expect(doc, name).toContain("axhub plugin-support tenant-resolve --json");
      expect(doc, name).toContain("needs_pick");
      expect(doc, name).toContain("candidates[]");
      // `.tenant // empty` 는 여러 작업공간에서 빈 문자열을 내고 needs_pick 을 숨겨요.
      expect(doc, name).not.toContain("tenant // empty");
    }
    expect(BOOTSTRAP).toContain(
      "`needs_pick:false` 면 `tenant`, `true` 면 `candidates[]` 로 `새 앱을 어느 작업공간에 만들까요?`라고 물어 고른 `slug`",
    );
    expect(SCAFFOLD).toContain("`needs_pick:true` 면 `candidates[]` 로 `새 앱을 어느 작업공간에 만들까요?` 를 물어 고른 `slug`");
    // preflight `current_team_id` 는 항상 null 이라 근거가 될 수 없어요.
    expect(RESUME).not.toContain("current_team_id");
    expect(RESUME).not.toContain("first-candidate fallback");
    expect(RESUME).toContain("Read `{tenant, source, needs_pick, candidates[]}`");
  });

  test("P2-42 scaffold never pushes into a repository that already existed", () => {
    const create = section(SCAFFOLD, "### 7. 저장소 생성 + push", "### 7.5. origin 등록");
    const executeLines = create.split("\n").filter((line) => line.startsWith("axhub github repo create") && line.includes("--execute"));

    expect(executeLines).toEqual(['axhub github repo create --owner "<owner>" --name "<slug>" --execute']);
    expect(create).toContain("`already_existed:true` 면 기존 저장소라 push 하지 말고 다른 저장소 이름을 물어요");
    expect(create).toContain('`false` 일 때만 `--push "<target>"` 를 붙여 다시 실행해요');
    expect(create.indexOf("`already_existed:true` 면")).toBeLessThan(create.indexOf('`--push "<target>"` 를 붙여'));
    expect(create).toContain("`저장소는 만들어졌는데 push 가 실패했어요` 면");
    expect(create).not.toContain("`name already exists` 는 다른 이름을 물어요");
    expect(SCAFFOLD).toContain("NEVER 첫 `--execute` 에 `--push` 를 붙이거나, 첫 실행이 `already_existed:true` 로 답한 저장소에 push·강제 push 하지 않아요");
  });

  test("P2-43 GitHub-blocked fallback confirms before apps create and stops headless", () => {
    const fallback = section(BOOTSTRAP, "### 12. GitHub 이 막혔을 때", "## NEVER");

    expect(fallback).not.toContain("**묻지 않고**");
    expect(fallback).not.toContain("바로 진행");
    expect(fallback).toContain("headless 는 만들거나 배포하지 않고");
    expect(fallback).toContain("설치 계정 0개는 이 갈래가 아니라 6단계 설치 확인(`GitHub App 설치를 끝냈을까요?`)");
    expect(fallback).toContain("`지금 만들고 배포까지 진행할까요?`(`진행`/`취소`)에 `진행`을 받은 뒤에만 만들어요");
    expect(fallback.indexOf("`지금 만들고 배포까지 진행할까요?`")).toBeLessThan(fallback.indexOf("axhub apps create"));
    expect(BOOTSTRAP).not.toContain("세 조건");
    expect(BOOTSTRAP).toContain("**단 12단계(GitHub 차단)에서 `진행` 확인을 받은 경우만 예외**");

    const createStep = section(BLOCKED, "**5. 앱 생성**", "**6. 배포**");
    expect(BLOCKED).not.toContain("**묻지 않고 진행하고**");
    expect(BLOCKED).toContain("설치 계정 0개는 여기로 오지 않아요");
    expect(BLOCKED).toContain("headless 면 아무것도 만들거나 배포하지 않고");
    expect(createStep).toContain("`지금 만들고 배포까지 진행할까요?`(`진행`/`취소`)");
    expect(createStep.indexOf("`진행` 을 고른 뒤에만")).toBeLessThan(createStep.indexOf("axhub apps create"));
    expect(BLOCKED).toContain("NEVER `지금 만들고 배포까지 진행할까요?` 에 `진행` 을 받기 전이나 headless 에서 `apps create` 를 실행하지 않아요");
  });

  test("P2-46 resume carries the chosen workspace and checks every candidate before starting over", () => {
    // init-resume state 와 그 status/resume 명령에는 작업공간이 없어요. skill 이 literal `--tenant` 를 붙여요.
    expect(BOOTSTRAP).toContain("resume state 엔 작업공간이 없어 status/resume 명령에도 이 값을 `--tenant` 로 붙이고");
    expect(BOOTSTRAP).toContain("못 찾으면(exit 5·8) 다른 `candidates` 를 다 본 뒤에만 새로 시작해요");
    expect(RESUME).toContain("the resume state (`.axhub/init-resume.json`) stores no workspace");
    expect(RESUME).toContain("append it as a literal `--tenant <tenant>` flag");
    expect(RESUME).toContain("- `watch_status`: run `args.status_command` with `--tenant <tenant>` appended");
    expect(RESUME).toContain("Always append `--tenant <tenant>` from the workspace carry above");
    expect(RESUME).not.toContain("only when `$AXHUB_TENANT` is set");
    expect(RESUME).not.toContain("stored template/name/slug/tenant/idempotency");

    const candidateCheck = RESUME.indexOf("Check each `candidates[].slug` in turn");
    const startOver = RESUME.indexOf("이전 기록을 찾지 못해서 새로 시작할게요.");
    expect(candidateCheck).toBeGreaterThan(-1);
    expect(candidateCheck).toBeLessThan(startOver);
    expect(RESUME).toContain("counts as not found only after every `candidates[].slug` was checked");

    // 존재하는 플래그만 써요: init-resume put 에는 tenant 옵션이 없어요.
    for (const [name, doc] of [
      ["bootstrap", BOOTSTRAP],
      ["bootstrap-and-local", LOCAL],
      ["resume-and-tenant", RESUME],
    ] as const) {
      for (const line of doc.split("\n").filter((l) => l.includes("init-resume put"))) {
        expect(line, name).not.toContain("--tenant");
      }
    }
  });

  test("P3-38 onboarding defines where <app> and <tenant> come from", () => {
    const gate = section(ONBOARDING, "### 2. Git backend gate", "### 3. Repository provider surface");

    expect(gate).toContain("app target 은 `axhub plugin-support resolve --json` 의 `app_id` 예요");
    expect(gate).toContain("`<tenant>` 는 `axhub plugin-support tenant-resolve --json` 의 `tenant`, `needs_pick:true` 면 `candidates[]` 에서 고른 `slug` 예요");
    // CLI·auth gap 과 preflight 뒤, backend read 앞에서 출처를 정해요.
    expect(gate.indexOf("axhub plugin-support preflight --json")).toBeLessThan(gate.indexOf("axhub plugin-support resolve --json"));
    expect(gate.indexOf("axhub plugin-support resolve --json")).toBeLessThan(gate.indexOf("axhub apps get <app> --json"));
  });

  test("P3-39 the onboarding ready-card template prints no internal state tag", () => {
    const card = section(READY_CARD, "## VIBE_READY Card", "## Degraded Cards");
    const template = section(card, "```text", "\n```");

    expect(template).toContain("axhub 온보딩 완료예요.\n");
    expect(template).not.toContain("[VIBE_READY]");
    expect(READY_CARD).not.toContain("[VIBE_READY]");
    // 내부 상태 이름은 heading·규칙에 남아요.
    expect(READY_CARD).toContain("VIBE_READY");
  });

  test("P3-40 onboarding auth reference matches the CLI auth contract", () => {
    const authMissing = section(INSTALL, "## Auth Missing", "## Auth Unverified");

    // 만료·미로그인은 subcode 없는 exit 4 `auth` 하나예요.
    expect(authMissing).toContain("exit 4 with `error.code` `auth` and no subcode");
    expect(authMissing).not.toContain("`code: token_expired`");
    expect(authMissing).not.toContain("`code: not_logged_in`");
    // pending 이벤트 이름은 `device_code_pending` 이고, `device_flow_pending` 은 종료 subcode 예요.
    expect(authMissing).toContain("- `device_code_pending` event (the run then exits with subcode `device_flow_pending`)");
    expect(authMissing).not.toContain("- `device_flow_pending`:");
    // AGENTS.md 의 최소 표면과 같아요.
    expect(INSTALL).toContain("This plugin needs ax-hub-cli v0.21.3+");
    expect(INSTALL).not.toContain("v0.20.0+");
  });

  test("P3-41 bootstrap references agree with SKILL.md on clone, watch flags, and step numbers", () => {
    // clone: SKILL.md 10단계 one-liner 처럼 main 을 직접 fetch 하고 FETCH_HEAD 로 맞춰요.
    expect(BOOTSTRAP).toContain("fetch origin main --quiet --depth=1");
    expect(BOOTSTRAP).toContain("reset --hard FETCH_HEAD");
    expect(LOCAL).toContain("  git fetch origin main --quiet --depth=1\n  git reset --hard FETCH_HEAD");
    expect(LOCAL).not.toContain("symbolic-ref");
    expect(LOCAL).not.toContain('reset --hard "origin/');

    // watch: bootstrap-status 의 CLI 내부 대기는 허용, 첫 execute/resume 만 금지.
    expect(BOOTSTRAP).toContain("이 명령에 `--watch --watch-timeout 9m` 을 붙여 이어가고");
    expect(RESUME).toContain("--watch --watch-timeout 9m");
    expect(LOCAL).toContain("--tenant test --watch --watch-timeout 9m --json");
    expect(LOCAL).toContain("Only the first execute and the first Desktop resume stay free of watch flags");
    expect(LOCAL).not.toContain("Do not use CLI watch flags from Claude Desktop");

    // execute 는 9단계예요.
    for (const [name, doc] of [
      ["bootstrap-and-local", LOCAL],
      ["errors-and-followups", ERRORS],
    ] as const) {
      expect(doc, name).not.toContain("Step 7");
      expect(doc, name).toContain("Step 9 execute");
    }
  });

  test("P3-44 bootstrap and scaffold reuse the backend onboarding remembered", () => {
    const backendGate = section(BOOTSTRAP, "### 6. Git Backend Gate", "### 7. Availability Check");

    expect(BOOTSTRAP).toContain("6. Git backend: persisted·`remembered` 선택을 재사용");
    expect(backendGate).toContain("`git_backend.source`·`remembered`만 읽어요");
    expect(backendGate).toContain("다음은 `remembered`(onboarding 이 `--remember` 로 남긴 값)");
    expect(backendGate.indexOf("`remembered`(onboarding")).toBeLessThan(backendGate.indexOf("이 앱의 코드 저장 위치를 선택해 주세요."));

    const storage = section(SCAFFOLD, "### 1.5 코드 저장 위치", "### 2. GitHub 계정 연동 확인");
    expect(storage).toContain("없으면 응답의 `remembered`(onboarding 이 기억한 값)가 `SELECTED_GIT_BACKEND` 예요");
    // 1.5단계에서는 템플릿·앱 이름을 아직 정하지 않았어요.
    expect(storage).toContain("`bootstrap` 에 선택값·작업공간만 넘겨요");
    expect(storage).toContain("템플릿·앱 이름은 아직 안 정했으니 bootstrap 이 묻고");
    expect(storage).not.toContain("정한 템플릿·앱 이름을 넘겨요");

    expect(ONBOARDING).toContain("다음 세션의 import·bootstrap·scaffold가 같은 저장 위치로 가요");
  });

  test("P1-13 bootstrap verify --wait fits the host tool timeout and ends interruptions with a resume summary", () => {
    for (const [name, doc] of [
      ["bootstrap", BOOTSTRAP],
      ["bootstrap-and-local", LOCAL],
    ] as const) {
      expect(doc, name).toContain("--wait --wait-interval 20s --wait-timeout 9m --json");
      expect(doc, name).not.toContain("--wait-timeout 10m");
      expect(doc, name).toContain("명령 도구 timeout 을 600000ms 로 올려 실행해요(CLI 대기 9분이 도구 한도 안에서 끝나요)");
      expect(doc, name).toContain("exit 6·10 이나 도구 timeout 으로 끊기면 실패로 말하지 않고, 재개 요약과 같은 verify 명령 한 줄만 남겨요");
    }
  });
});
