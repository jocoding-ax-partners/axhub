import { describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { chmodSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const repoRoot = join(import.meta.dir, "..");
const helperBinary = join(repoRoot, "target", "debug", "axhub-helpers");

function ensureHelperBuilt() {
  const build = spawnSync("cargo", ["build", "-p", "axhub-helpers"], {
    cwd: repoRoot,
    encoding: "utf8",
    timeout: 60_000,
  });
  expect(build.status).toBe(0);
}

function fakeAxhub(dir: string): string {
  const bin = join(dir, "axhub");
  writeFileSync(
    bin,
    `#!/usr/bin/env bash
set -euo pipefail
if [ "\${1:-}" = "--version" ]; then
  echo "axhub 0.17.4"
  exit 0
fi
if [ "\${1:-}" = "auth" ] && [ "\${2:-}" = "status" ] && [ "\${3:-}" = "--json" ]; then
  echo '{"user_email":"qa@example.test","expires_at":"2099-01-01T00:00:00Z","scopes":["read","deploy"]}'
  exit 0
fi
echo '{"ok":true}'
`,
  );
  chmodSync(bin, 0o755);
  return bin;
}

function promptRoute(prompt: string): string {
  ensureHelperBuilt();
  const dir = mkdtempSync(join(tmpdir(), "axhub-prompt-route-karpathy-"));
  const out = spawnSync(helperBinary, ["prompt-route"], {
    cwd: dir,
    input: JSON.stringify({ hook_event_name: "UserPromptSubmit", prompt }),
    env: {
      ...process.env,
      AXHUB_BIN: fakeAxhub(dir),
      CLAUDE_PLUGIN_ROOT: repoRoot,
      XDG_STATE_HOME: join(dir, "state"),
    },
    encoding: "utf8",
    timeout: 10_000,
  });
  expect(out.status).toBe(0);
  return out.stdout;
}

describe("prompt-route Karpathy context gating", () => {
  test("first-run vibe prompts receive onboarding contract instead of generic advice", () => {
    const stdout = promptRoute("처음인데 뭐부터 하면 돼?");
    const payload = JSON.parse(stdout);
    const additionalContext = payload.hookSpecificOutput?.additionalContext ?? "";
    const systemMessage = payload.systemMessage ?? "";
    const routedText = [
      additionalContext,
      systemMessage,
    ].join("\n");
    expect(additionalContext).toContain('Skill("axhub:onboarding")');
    expect(systemMessage).toContain('Skill("axhub:onboarding")');
    expect(routedText).toContain("처음 설정을 확인할게요");
    expect(routedText).toContain("first-run onboarding");
    expect(routedText).toContain("첫 앱 만들래요?");
    expect(routedText).toContain("never announce internal routing");
    expect(systemMessage).toContain("내부 제어");
    expect(systemMessage).toContain("visible chat 에 절대 쓰지 않아요");
    expect(systemMessage).toContain("내부 실행 선언");
    expect(routedText).toContain("VIBE_READY");
    expect(routedText).not.toContain("새 앱 만들어줘\" 하면 됨");
    expect(routedText).not.toContain("I'll invoke the onboarding skill");
  });

  test("short new-app prompts receive Desktop init contract instead of generic app ideation", () => {
    const stdout = promptRoute("새 앱 만들어줘");
    expect(stdout).toContain("새 앱을 만들 수 있는 템플릿을 확인할게요");
    expect(stdout).toContain("AXHub app creation request");
    expect(stdout).toContain("init-resume route --json");
    expect(stdout).toContain("FastAPI");
    expect(stdout).toContain("Next.js");
    expect(stdout).toContain("빈 템플릿");
    expect(stdout).toContain("Do not add an explicit 기타 option");
    expect(stdout).toContain("Do not offer generic choices");
    expect(stdout).toContain("axhub apps bootstrap --execute");
    expect(stdout).not.toContain("무슨 앱 만들래");
  });

  test("doctor status prompts receive doctor skill contract instead of generic diagnostics", () => {
    const stdout = promptRoute("axhub CLI 설치 상태 괜찮아?");
    const payload = JSON.parse(stdout);
    const additionalContext = payload.hookSpecificOutput?.additionalContext ?? "";
    const systemMessage = payload.systemMessage ?? "";
    const routedText = [additionalContext, systemMessage].join("\n");
    expect(additionalContext).toContain('Skill("axhub:doctor")');
    expect(systemMessage).toContain('Skill("axhub:doctor")');
    expect(routedText).toContain("설치 상태를 확인할게요");
    expect(routedText).toContain("doctor-summary --user-utterance");
    expect(routedText).toContain("Do not install, update, login, logout");
    expect(routedText).toContain("visible chat 에 절대 쓰지 않아요");
    expect(routedText).not.toContain("/axhub:doctor");
    expect(routedText).not.toContain("I'll invoke the doctor skill");
  });

  test("ordinary deploy prompts do not receive unrelated Karpathy skill body", () => {
    const stdout = promptRoute("내 paydrop 앱 배포해");
    expect(stdout).toContain("배포 준비를 확인할게요");
    expect(stdout).toContain("deploy-preview-summary");
    expect(stdout).toContain("deploy-approved-run");
    expect(stdout).toContain("사용자에게 보이는 앱 폴더");
    expect(stdout).toContain("axhub 매니페스트(axhub.yaml)가 없어요.");
    expect(stdout).toContain("React/Vite로 초기화");
    expect(stdout).toContain("Invoke deploy skill");
    expect(stdout).toContain("consent token");
    expect(stdout).not.toContain("AXHub deploy workflow");
    expect(stdout).not.toContain("Skill(axhub:");
    expect(stdout).not.toContain("karpathy-guidelines");
    expect(stdout).not.toContain("Keep changes small and inspectable");
  });

  test("explicit coding reminder prompt still receives Karpathy context", () => {
    const stdout = promptRoute("테스트 우선으로 작은 diff 로 가");
    expect(stdout).toContain("karpathy-guidelines");
    expect(stdout).toContain("Keep changes small and inspectable");
  });
});

describe("prompt-route direct quality workflow gating", () => {
  test("pure code review phrasing receives dedicated review contract", () => {
    const stdout = promptRoute("이 코드 리뷰해줘");
    expect(stdout).toContain("코드 리뷰를 시작할게요");
    expect(stdout).toContain("review-scope-summary");
    expect(stdout).toContain("변경 범위 확인");
    expect(stdout).toContain("변경량이 커서 먼저 범위를 정할게요");
    expect(stdout).toContain("리뷰 상태 저장");
    expect(stdout).not.toContain("systemMessage");
    expect(stdout).not.toContain("axhub hook");
    expect(stdout).not.toContain("Control only");
    expect(stdout).not.toContain("Survey repo scope");
    expect(stdout).not.toContain("실제 코드 파일 스캔");
    expect(stdout).not.toContain("실제 코드 파일 읽기");
    expect(stdout).not.toContain("Process the user's primary task normally");
  });

  test("quality neighbors route to their dedicated contracts", () => {
    const cases = [
      ["왜 테스트가 깨지는지 디버그해줘", "원인을 좁혀볼게요", "디버그 상태 저장"],
      ["loop 돌려서 원인 찾아줘", "진단 루프를 준비할게요", "진단 루프 준비"],
      ["큰 구조 변경 계획 세워줘", "변경 계획을 잡아볼게요", "영향 범위 확인"],
      ["PR 만들기 전에 배포 준비 봐줘", "출시 준비 상태를 확인할게요", "출시 상태 저장"],
      ["테스트 먼저 TDD로 가자", "테스트부터 잡아볼게요", "TDD 대상 확인"],
    ] as const;

    for (const [prompt, firstSentence, toolTitle] of cases) {
      const stdout = promptRoute(prompt);
      expect(stdout, prompt).toContain(firstSentence);
      expect(stdout, prompt).toContain(toolTitle);
      expect(stdout, prompt).toContain("Never display account email");
      expect(stdout, prompt).not.toContain("Process the user's primary task normally");
    }
  });

  test("marketplace review phrasing is publish, not code review", () => {
    const stdout = promptRoute("이 앱 공개 심사 넣고 싶어");
    expect(stdout).toContain("공개 심사 준비를 확인할게요");
    expect(stdout).toContain("publish-summary");
    expect(stdout).not.toContain("코드 리뷰를 시작할게요");
    expect(stdout).not.toContain("direct code-review request");
  });

  test("login-needed phrasing routes to auth without log detour", () => {
    const stdout = promptRoute("로그인 다시 해야 해?");
    expect(stdout).toContain("로그인 상태를 확인할게요");
    expect(stdout).toContain("auth-summary --user-utterance");
    expect(stdout).toContain("다시 로그인 필요 여부만 확인");
    expect(stdout).not.toContain("systemMessage");
    expect(stdout).not.toContain("logs-summary --user-utterance");
    expect(stdout).not.toContain("로그를 확인할게요");
    expect(stdout).not.toContain("Control only");
    expect(stdout).not.toContain("/axhub:");
  });
});
