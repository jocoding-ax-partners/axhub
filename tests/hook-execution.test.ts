import { describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, utimesSync, writeFileSync, chmodSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

// 훅 bash 를 실제로 실행하는 하네스예요 — 문자열 계약 테스트가 못 잡는 의미
// 회귀(F1: whole-JSON 매칭 오탐)를 잠가요. 합성 payload 는 공식 문서 예시
// 스키마를 미러링해요 (code.claude.com/docs/en/hooks — session_id →
// transcript_path → cwd → permission_mode → hook_event_name → prompt).
// 인라인 command 는 운영과 동일하게 plain `bash -c` 로 실행해요 (set -u 주입
// 금지). spawn timeout 은 성능 assert 가 아니라 deadlock 가드예요.

const REPO_ROOT = join(import.meta.dir, "..");
const SPAWN_TIMEOUT_MS = 10_000;

const hooksFile = JSON.parse(readFileSync(join(REPO_ROOT, "hooks", "hooks.json"), "utf8")) as {
  hooks: {
    UserPromptSubmit: Array<{ hooks: Array<{ command: string }> }>;
    SessionStart: Array<{ hooks: Array<{ command: string }> }>;
  };
};
const upsCommands = hooksFile.hooks.UserPromptSubmit.flatMap((group) => group.hooks).map((h) => h.command);
const ssCommands = hooksFile.hooks.SessionStart.flatMap((group) => group.hooks).map((h) => h.command);
const updateRouterCmd = upsCommands[0]!;
const autoUpdateCmd = ssCommands[0]!;
const onboardingResumeCmd = ssCommands[1]!;
const windowsContractCmd = ssCommands[2]!;
const ap14FallbackCmd = ssCommands[3]!;
const restartConfirmCmd = ssCommands[4]!;
const feedbackContractCmd = ssCommands[5]!;

const DEV_CWD = "/Users/dev/work/jocoding/axhub"; // 경로에 axhub 포함 (F1 재현용)

interface RunResult {
  stdout: string;
  exitCode: number;
}

// 라우터·훅은 $HOME/.axhub/config/no-* marker kill switch 를 읽으므로,
// 개발 머신의 실제 marker 가 결과를 흔들지 않게 기본 HOME 은 격리된 빈
// 디렉토리를 써요. marker 양성 케이스는 makeHomeWithMarker 로 명시해요.
const ISOLATED_HOME = mkdtempSync(join(tmpdir(), "axhub-hook-isolated-home-"));

function baseEnv(extra: Record<string, string> = {}): Record<string, string> {
  return {
    PATH: process.env.PATH ?? "/usr/bin:/bin",
    HOME: ISOLATED_HOME,
    ...extra,
  };
}

function makeHomeWithMarker(name: string): string {
  const home = mkdtempSync(join(tmpdir(), "axhub-hook-marker-home-"));
  mkdirSync(join(home, ".axhub", "config"), { recursive: true });
  writeFileSync(join(home, ".axhub", "config", `no-${name}`), "");
  return home;
}

function runBash(argv: string[], stdin: string, env: Record<string, string>): RunResult {
  const result = Bun.spawnSync({
    cmd: argv,
    stdin: Buffer.from(stdin),
    stdout: "pipe",
    stderr: "pipe",
    env,
    timeout: SPAWN_TIMEOUT_MS,
  });
  return { stdout: result.stdout.toString(), exitCode: result.exitCode ?? -1 };
}

function runInline(command: string, stdin: string, env: Record<string, string> = {}): RunResult {
  return runBash(["bash", "-c", command], stdin, baseEnv(env));
}

function promptPayload(prompt: string, cwd = "/Users/dev/projects/demo"): string {
  return JSON.stringify({
    session_id: "abc123",
    transcript_path: `${cwd.replaceAll("/", "-")}/00893aaf.jsonl`,
    cwd,
    permission_mode: "default",
    hook_event_name: "UserPromptSubmit",
    prompt,
  });
}

// v2.1.196+ 실페이로드 변형: 공통 필드 prompt_id 가 prompt 앞에 존재해요.
function promptIdPayload(prompt: string, cwd = "/Users/dev/projects/demo"): string {
  return JSON.stringify({
    session_id: "abc123",
    prompt_id: "p-001",
    transcript_path: `${cwd.replaceAll("/", "-")}/00893aaf.jsonl`,
    cwd,
    permission_mode: "default",
    hook_event_name: "UserPromptSubmit",
    prompt,
  });
}

function noPromptKeyPayload(cwd = DEV_CWD): string {
  return JSON.stringify({
    session_id: "abc123",
    transcript_path: `${cwd.replaceAll("/", "-")}/00893aaf.jsonl`,
    cwd,
    hook_event_name: "UserPromptSubmit",
  });
}

function expectSilent(result: RunResult): void {
  expect(result.exitCode).toBe(0);
  expect(result.stdout.trim()).toBe("");
}

function expectEmit(result: RunResult, eventName: string, marker: string): void {
  expect(result.exitCode).toBe(0);
  const line = result.stdout.trim();
  expect(line.length).toBeGreaterThan(0);
  const parsed = JSON.parse(line) as {
    continue: boolean;
    suppressOutput: boolean;
    hookSpecificOutput: { hookEventName: string; additionalContext: string };
  };
  expect(parsed.continue).toBe(true);
  expect(parsed.suppressOutput).toBe(true);
  expect(parsed.hookSpecificOutput.hookEventName).toBe(eventName);
  expect(parsed.hookSpecificOutput.additionalContext).toContain(marker);
}

const routerEnv = { CLAUDE_PLUGIN_ROOT: REPO_ROOT };
const runUpdateRouter = (stdin: string, env: Record<string, string> = {}) =>
  runInline(updateRouterCmd, stdin, { ...routerEnv, ...env });

describe("update-router (UserPromptSubmit entry 1 → hooks/update-router.sh)", () => {
  test("prompt 의 axhub+freshness 발화에 발동해요", () => {
    expectEmit(runUpdateRouter(promptPayload("axhub 업데이트해줘")), "UserPromptSubmit", "[axhub update router]");
  });

  test("F1 회귀 잠금: cwd/transcript_path 에만 axhub 가 있으면 침묵해요 (CRITICAL)", () => {
    expectSilent(runUpdateRouter(promptPayload("npm 업데이트 해줘", DEV_CWD)));
  });

  test('"prompt": 키 부재 payload 는 fail-closed 로 침묵해요', () => {
    expectSilent(runUpdateRouter(noPromptKeyPayload()));
  });

  test("prompt_id 포함 변형(v2.1.196+)에서도 정상 발동해요", () => {
    expectEmit(runUpdateRouter(promptIdPayload("axhub 최신인지 확인해줘")), "UserPromptSubmit", "[axhub update router]");
  });

  test("문두 자동 대문자화 변형 Axhub 에 발동해요 (E4)", () => {
    expectEmit(runUpdateRouter(promptPayload("Axhub 최신인지 확인해줘")), "UserPromptSubmit", "현재 버전을 확인할게요");
  });

  test("casing 경계: freshness 키워드는 소문자 전용이라 Update 대문자 발화엔 침묵해요 (의도된 경계)", () => {
    expectSilent(runUpdateRouter(promptPayload("Update axhub 해줘")));
  });

  test("freshness 없는 axhub 발화(자기-트리거 음성)엔 침묵해요", () => {
    expectSilent(runUpdateRouter(promptPayload("axhub 로그 좀 보여줘")));
  });

  test("kill switch AXHUB_NO_UPDATE_ROUTER 가 있으면 침묵해요", () => {
    expectSilent(runUpdateRouter(promptPayload("axhub 업데이트해줘"), { AXHUB_NO_UPDATE_ROUTER: "1" }));
  });

  test("marker kill switch no-update-router 가 있으면 침묵해요", () => {
    expectSilent(runUpdateRouter(promptPayload("axhub 업데이트해줘"), { HOME: makeHomeWithMarker("update-router") }));
  });
});

describe("UserPromptSubmit 라우터 diet (update 단독)", () => {
  test("운영 발화(로그·테이블·환경변수)엔 어떤 라우터도 발동하지 않아요 — clarity frontmatter 라우팅 소관", () => {
    for (const prompt of ["axhub 로그 보여줘", "axhub 테이블 만들어줘", "axhub 환경변수 확인해줘"]) {
      expectSilent(runUpdateRouter(promptPayload(prompt)));
    }
  });

  test("UserPromptSubmit 표면은 update 라우터 1개뿐이에요 (AGENTS diet 허용 목록)", () => {
    expect(upsCommands).toHaveLength(1);
    expect(upsCommands[0]).toContain("update-router.sh");
  });

  test("제거된 라우터 스크립트는 hooks/ 에 존재하지 않아요", () => {
    for (const script of ["clarity-router.sh", "import-router.sh", "status-resume-router.sh"]) {
      expect(existsSync(join(REPO_ROOT, "hooks", script))).toBe(false);
    }
  });

  test("freshness 발화 → update 라우터가 발동해요", () => {
    expectEmit(runUpdateRouter(promptPayload("axhub 최신 버전으로 업데이트해줘")), "UserPromptSubmit", "[axhub update router]");
  });

  test("F1 재현 payload (axhub 는 경로에만) → 침묵해요", () => {
    expectSilent(runUpdateRouter(promptPayload("이 프로젝트 상태 확인하고 기존 코드 업데이트해줘", DEV_CWD)));
  });
});

// ── SessionStart 훅 ──────────────────────────────────────────────────────

function makeHome(): string {
  return mkdtempSync(join(tmpdir(), "axhub-hook-home-"));
}

function makeStubAxhubPath(): string {
  const bin = mkdtempSync(join(tmpdir(), "axhub-stub-bin-"));
  const stub = join(bin, "axhub");
  writeFileSync(stub, "#!/bin/sh\nexit 0\n");
  chmodSync(stub, 0o755);
  return bin;
}

const STUB_BIN = makeStubAxhubPath();
const SAFE_PATH = `${STUB_BIN}:/usr/bin:/bin`;
const NO_AXHUB_PATH = "/usr/bin:/bin";
const CACHE_REL = join(".axhub", "cache", ".plugin-update-check");
const MARKER_REL = join(".axhub", "cache", ".plugin-update-restart");

function daysAgo(days: number): Date {
  return new Date(Date.now() - days * 24 * 60 * 60 * 1000);
}

describe("auto-update 훅 (SessionStart entry 1)", () => {
  test("캐시 없음 → context 발행 + 캐시 파일을 훅이 직접 생성해요 (touch-in-hook)", () => {
    const home = makeHome();
    const result = runInline(autoUpdateCmd, "", { HOME: home, PATH: SAFE_PATH });
    expectEmit(result, "SessionStart", "auto-update-prompt.md");
    expect(existsSync(join(home, CACHE_REL))).toBe(true);
  });

  test("fresh 캐시(24h 이내) → 침묵해요", () => {
    const home = makeHome();
    mkdirSync(join(home, ".axhub", "cache"), { recursive: true });
    writeFileSync(join(home, CACHE_REL), "");
    expectSilent(runInline(autoUpdateCmd, "", { HOME: home, PATH: SAFE_PATH }));
  });

  test("stale 캐시(24h 초과) → 발행하고 캐시를 갱신해요", () => {
    const home = makeHome();
    mkdirSync(join(home, ".axhub", "cache"), { recursive: true });
    const cache = join(home, CACHE_REL);
    writeFileSync(cache, "");
    utimesSync(cache, daysAgo(2), daysAgo(2));
    expectEmit(runInline(autoUpdateCmd, "", { HOME: home, PATH: SAFE_PATH }), "SessionStart", "auto-update-prompt.md");
  });

  test("kill switch AXHUB_NO_AUTO_UPDATE → 침묵 + 캐시도 만들지 않아요", () => {
    const home = makeHome();
    expectSilent(runInline(autoUpdateCmd, "", { HOME: home, PATH: SAFE_PATH, AXHUB_NO_AUTO_UPDATE: "1" }));
    expect(existsSync(join(home, CACHE_REL))).toBe(false);
  });

  test("marker kill switch no-auto-update → 침묵 + 캐시도 만들지 않아요", () => {
    const home = makeHomeWithMarker("auto-update");
    expectSilent(runInline(autoUpdateCmd, "", { HOME: home, PATH: SAFE_PATH }));
    expect(existsSync(join(home, CACHE_REL))).toBe(false);
  });

  test("axhub 바이너리 부재 → 침묵해요", () => {
    const home = makeHome();
    expectSilent(runInline(autoUpdateCmd, "", { HOME: home, PATH: NO_AXHUB_PATH }));
    expect(existsSync(join(home, CACHE_REL))).toBe(false);
  });

  test("dev 가드 토폴로지 A: <repo>/plugins/axhub in-place 로딩 → 침묵해요", () => {
    const home = makeHome();
    const repo = mkdtempSync(join(tmpdir(), "axhub-dev-repo-"));
    mkdirSync(join(repo, ".git"), { recursive: true });
    const pluginRoot = join(repo, "plugins", "axhub");
    mkdirSync(pluginRoot, { recursive: true });
    expectSilent(runInline(autoUpdateCmd, "", { HOME: home, PATH: SAFE_PATH, CLAUDE_PLUGIN_ROOT: pluginRoot }));
    expect(existsSync(join(home, CACHE_REL))).toBe(false);
  });

  test("dev 가드 토폴로지 B: repo 루트 직접 로딩 (루트 plugin.json 레이아웃) → 침묵해요", () => {
    const home = makeHome();
    const repo = mkdtempSync(join(tmpdir(), "axhub-dev-root-"));
    mkdirSync(join(repo, ".git"), { recursive: true });
    expectSilent(runInline(autoUpdateCmd, "", { HOME: home, PATH: SAFE_PATH, CLAUDE_PLUGIN_ROOT: repo }));
    expect(existsSync(join(home, CACHE_REL))).toBe(false);
  });
});

describe("restart-confirm 훅 (SessionStart entry 5)", () => {
  test("fresh marker → confirm prompt 지시를 발행해요", () => {
    const home = makeHome();
    mkdirSync(join(home, ".axhub", "cache"), { recursive: true });
    writeFileSync(join(home, MARKER_REL), "1.10.28");
    expectEmit(runInline(restartConfirmCmd, "", { HOME: home }), "SessionStart", "plugin-restart-confirm-prompt.md");
  });

  test("marker 없음 → 침묵해요", () => {
    expectSilent(runInline(restartConfirmCmd, "", { HOME: makeHome() }));
  });

  test("TTL(7일) 초과 marker → 침묵해요 (휴면)", () => {
    const home = makeHome();
    mkdirSync(join(home, ".axhub", "cache"), { recursive: true });
    const marker = join(home, MARKER_REL);
    writeFileSync(marker, "1.10.28");
    utimesSync(marker, daysAgo(8), daysAgo(8));
    expectSilent(runInline(restartConfirmCmd, "", { HOME: home }));
  });

  test("kill switch AXHUB_NO_AUTO_UPDATE → 침묵해요", () => {
    const home = makeHome();
    mkdirSync(join(home, ".axhub", "cache"), { recursive: true });
    writeFileSync(join(home, MARKER_REL), "1.10.28");
    expectSilent(runInline(restartConfirmCmd, "", { HOME: home, AXHUB_NO_AUTO_UPDATE: "1" }));
  });

  test("marker kill switch no-auto-update → 침묵해요 (auto-update 계열 공용)", () => {
    const home = makeHomeWithMarker("auto-update");
    mkdirSync(join(home, ".axhub", "cache"), { recursive: true });
    writeFileSync(join(home, MARKER_REL), "1.10.28");
    expectSilent(runInline(restartConfirmCmd, "", { HOME: home }));
  });
});

describe("onboarding resume 훅 (SessionStart entry 2)", () => {
  const RESUME_REL = join(".axhub", "cache", ".onboarding-mcp-restart");

  test("fresh marker → resume 지시를 발행해요", () => {
    const home = makeHome();
    mkdirSync(join(home, ".axhub", "cache"), { recursive: true });
    writeFileSync(join(home, RESUME_REL), "");
    expectEmit(runInline(onboardingResumeCmd, "", { HOME: home }), "SessionStart", "mcp-ready-card.md");
  });

  test("marker 없음 → 침묵해요", () => {
    expectSilent(runInline(onboardingResumeCmd, "", { HOME: makeHome() }));
  });

  test("TTL(7일) 초과 → 침묵해요", () => {
    const home = makeHome();
    mkdirSync(join(home, ".axhub", "cache"), { recursive: true });
    const marker = join(home, RESUME_REL);
    writeFileSync(marker, "");
    utimesSync(marker, daysAgo(8), daysAgo(8));
    expectSilent(runInline(onboardingResumeCmd, "", { HOME: home }));
  });

  test("kill switch AXHUB_NO_ONBOARDING_RESUME → 침묵해요", () => {
    const home = makeHome();
    mkdirSync(join(home, ".axhub", "cache"), { recursive: true });
    writeFileSync(join(home, RESUME_REL), "");
    expectSilent(runInline(onboardingResumeCmd, "", { HOME: home, AXHUB_NO_ONBOARDING_RESUME: "1" }));
  });

  test("marker kill switch no-onboarding-resume → 침묵해요", () => {
    const home = makeHomeWithMarker("onboarding-resume");
    mkdirSync(join(home, ".axhub", "cache"), { recursive: true });
    writeFileSync(join(home, RESUME_REL), "");
    expectSilent(runInline(onboardingResumeCmd, "", { HOME: home }));
  });
});

describe("AP-13 Windows 계약 훅 (SessionStart entry 3)", () => {
  test("OS=Windows_NT → 계약을 발행해요", () => {
    expectEmit(runInline(windowsContractCmd, "", { OS: "Windows_NT" }), "SessionStart", "Git Bash");
  });

  test("non-Windows(OS 미설정) → 침묵해요", () => {
    expectSilent(runInline(windowsContractCmd, ""));
  });

  test("kill switch AXHUB_NO_WINDOWS_CONTRACT → 침묵해요", () => {
    expectSilent(runInline(windowsContractCmd, "", { OS: "Windows_NT", AXHUB_NO_WINDOWS_CONTRACT: "1" }));
  });

  test("marker kill switch no-windows-contract → 침묵해요", () => {
    expectSilent(runInline(windowsContractCmd, "", { OS: "Windows_NT", HOME: makeHomeWithMarker("windows-contract") }));
  });
});

describe("AP-19 feedback 리포트 계약 훅 (SessionStart entry 6)", () => {
  test("PATH 에 axhub 존재 → 계약을 발행해요", () => {
    const result = runInline(feedbackContractCmd, "", { HOME: makeHome(), PATH: SAFE_PATH });
    expectEmit(result, "SessionStart", "axhub feedback -m");
    expect(result.stdout).toContain("예상된 거절은 리포트하지 않아요");
  });

  test("PATH 에 없어도 위치 파일 ~/.axhub/bin-path 가 있으면 발행해요 (AP-17 3-경로)", () => {
    const home = makeHome();
    mkdirSync(join(home, ".axhub"), { recursive: true });
    writeFileSync(join(home, ".axhub", "bin-path"), "/opt/axhub/bin/axhub");
    expectEmit(runInline(feedbackContractCmd, "", { HOME: home, PATH: NO_AXHUB_PATH }), "SessionStart", "axhub feedback -m");
  });

  test("canonical ~/.axhub/bin/axhub 만 있어도 발행해요 (AP-17 3-경로)", () => {
    const home = makeHome();
    mkdirSync(join(home, ".axhub", "bin"), { recursive: true });
    writeFileSync(join(home, ".axhub", "bin", "axhub"), "");
    expectEmit(runInline(feedbackContractCmd, "", { HOME: home, PATH: NO_AXHUB_PATH }), "SessionStart", "axhub feedback -m");
  });

  test("CLI 가 3-경로 어디에도 없으면 침묵해요", () => {
    expectSilent(runInline(feedbackContractCmd, "", { HOME: makeHome(), PATH: NO_AXHUB_PATH }));
  });

  test("kill switch AXHUB_NO_FEEDBACK_REPORT → 침묵해요", () => {
    expectSilent(runInline(feedbackContractCmd, "", { HOME: makeHome(), PATH: SAFE_PATH, AXHUB_NO_FEEDBACK_REPORT: "1" }));
  });

  test("marker kill switch no-feedback-report → 침묵해요", () => {
    expectSilent(runInline(feedbackContractCmd, "", { HOME: makeHomeWithMarker("feedback-report"), PATH: SAFE_PATH }));
  });
});

describe("AP-14 폴백 훅 (SessionStart entry 4)", () => {
  test("기본 상태에서 update-first 가드를 발행해요", () => {
    expectEmit(runInline(ap14FallbackCmd, ""), "SessionStart", "update-first routing guard is active for Code mode");
  });

  test("kill switch AXHUB_NO_UPDATE_ROUTER → 침묵해요", () => {
    expectSilent(runInline(ap14FallbackCmd, "", { AXHUB_NO_UPDATE_ROUTER: "1" }));
  });

  test("marker kill switch no-update-router → 침묵해요 (UserPromptSubmit 라우터와 공용)", () => {
    expectSilent(runInline(ap14FallbackCmd, "", { HOME: makeHomeWithMarker("update-router") }));
  });
});
