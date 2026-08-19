import { describe, expect, test } from "bun:test";
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, utimesSync, writeFileSync, chmodSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { HOST_EXPECTATIONS, SESSION_WRAPPER_SCRIPTS } from "./fixtures/host-expectations";

// 훅 bash 를 실제로 실행하는 하네스예요 — 문자열 계약 테스트가 못 잡는 의미
// 회귀(F1: whole-JSON 매칭 오탐)를 잠가요. 합성 payload 는 공식 문서 예시
// 스키마를 미러링해요 (code.claude.com/docs/en/hooks — session_id →
// transcript_path → cwd → permission_mode → hook_event_name → prompt).
// SessionStart 5개 entry 는 hooks/session-*.sh wrapper 로 추출됐어요 (KTD6) —
// wrapper 는 운영과 동일하게 `bash "<path>"` 로 실행하고, wrapper 본문은 인라인
// 시절과 같은 plain bash 의미를 유지해요 (set -u 주입 금지). spawn timeout 은
// 성능 assert 가 아니라 deadlock 가드예요.

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

// hooks.json 의 SessionStart entry 순서와 1:1 인 wrapper 파일 목록은
// fixtures/host-expectations.ts 의 SESSION_WRAPPER_SCRIPTS 가 단일 소유해요.
const wrapperPath = (name: (typeof SESSION_WRAPPER_SCRIPTS)[number]): string => join(REPO_ROOT, "hooks", name);
const autoUpdateSh = wrapperPath("session-auto-update.sh");
const windowsContractSh = wrapperPath("session-windows-contract.sh");
const ap14FallbackSh = wrapperPath("session-update-router-guard.sh");
const restartConfirmSh = wrapperPath("session-restart-confirm.sh");
const feedbackContractSh = wrapperPath("session-feedback-contract.sh");

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

// SessionStart wrapper 는 운영 위임 형태 그대로 `bash "<path>"` 로 실행해요.
// CLAUDE_PLUGIN_ROOT 는 인라인 시절과 동일하게 기본 미설정이에요 — repo
// checkout 의 .git 이 dev 가드를 오발동시키지 않게 하고, dev 가드 양성
// 케이스는 각 테스트가 명시적으로 env 를 줘요.
function runScript(scriptPath: string, stdin: string, env: Record<string, string> = {}): RunResult {
  return runBash(["bash", scriptPath], stdin, baseEnv(env));
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

describe("SessionStart wrapper 위임 (KTD6: 인라인 command → hooks/session-*.sh)", () => {
  test("5개 entry 가 순서대로 각 wrapper 파일로 bare 위임해요 — 실행 대상과 hooks.json 이 일치해요", () => {
    expect(ssCommands).toEqual(SESSION_WRAPPER_SCRIPTS.map((name) => HOST_EXPECTATIONS.claude.surface.hookWrapperCommand(name)));
    for (const name of SESSION_WRAPPER_SCRIPTS) {
      expect(existsSync(wrapperPath(name))).toBe(true);
    }
  });
});

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
    const result = runScript(autoUpdateSh, "", { HOME: home, PATH: SAFE_PATH });
    expectEmit(result, "SessionStart", "auto-update-prompt.md");
    expect(existsSync(join(home, CACHE_REL))).toBe(true);
  });

  test("fresh 캐시(24h 이내) → 침묵해요", () => {
    const home = makeHome();
    mkdirSync(join(home, ".axhub", "cache"), { recursive: true });
    writeFileSync(join(home, CACHE_REL), "");
    expectSilent(runScript(autoUpdateSh, "", { HOME: home, PATH: SAFE_PATH }));
  });

  test("stale 캐시(24h 초과) → 발행하고 캐시를 갱신해요", () => {
    const home = makeHome();
    mkdirSync(join(home, ".axhub", "cache"), { recursive: true });
    const cache = join(home, CACHE_REL);
    writeFileSync(cache, "");
    utimesSync(cache, daysAgo(2), daysAgo(2));
    expectEmit(runScript(autoUpdateSh, "", { HOME: home, PATH: SAFE_PATH }), "SessionStart", "auto-update-prompt.md");
  });

  test("kill switch AXHUB_NO_AUTO_UPDATE → 침묵 + 캐시도 만들지 않아요", () => {
    const home = makeHome();
    expectSilent(runScript(autoUpdateSh, "", { HOME: home, PATH: SAFE_PATH, AXHUB_NO_AUTO_UPDATE: "1" }));
    expect(existsSync(join(home, CACHE_REL))).toBe(false);
  });

  test("marker kill switch no-auto-update → 침묵 + 캐시도 만들지 않아요", () => {
    const home = makeHomeWithMarker("auto-update");
    expectSilent(runScript(autoUpdateSh, "", { HOME: home, PATH: SAFE_PATH }));
    expect(existsSync(join(home, CACHE_REL))).toBe(false);
  });

  test("axhub 바이너리 부재 → 침묵해요", () => {
    const home = makeHome();
    expectSilent(runScript(autoUpdateSh, "", { HOME: home, PATH: NO_AXHUB_PATH }));
    expect(existsSync(join(home, CACHE_REL))).toBe(false);
  });

  test("dev 가드 토폴로지 A: <repo>/plugins/axhub in-place 로딩 → 침묵해요", () => {
    const home = makeHome();
    const repo = mkdtempSync(join(tmpdir(), "axhub-dev-repo-"));
    mkdirSync(join(repo, ".git"), { recursive: true });
    const pluginRoot = join(repo, "plugins", "axhub");
    mkdirSync(pluginRoot, { recursive: true });
    expectSilent(runScript(autoUpdateSh, "", { HOME: home, PATH: SAFE_PATH, CLAUDE_PLUGIN_ROOT: pluginRoot }));
    expect(existsSync(join(home, CACHE_REL))).toBe(false);
  });

  test("dev 가드 토폴로지 B: repo 루트 직접 로딩 (루트 plugin.json 레이아웃) → 침묵해요", () => {
    const home = makeHome();
    const repo = mkdtempSync(join(tmpdir(), "axhub-dev-root-"));
    mkdirSync(join(repo, ".git"), { recursive: true });
    expectSilent(runScript(autoUpdateSh, "", { HOME: home, PATH: SAFE_PATH, CLAUDE_PLUGIN_ROOT: repo }));
    expect(existsSync(join(home, CACHE_REL))).toBe(false);
  });
});

describe("restart-confirm 훅 (SessionStart entry 4)", () => {
  test("fresh marker → confirm prompt 지시를 발행해요", () => {
    const home = makeHome();
    mkdirSync(join(home, ".axhub", "cache"), { recursive: true });
    writeFileSync(join(home, MARKER_REL), "1.10.28");
    expectEmit(runScript(restartConfirmSh, "", { HOME: home }), "SessionStart", "plugin-restart-confirm-prompt.md");
  });

  test("marker 없음 → 침묵해요", () => {
    expectSilent(runScript(restartConfirmSh, "", { HOME: makeHome() }));
  });

  test("TTL(7일) 초과 marker → 침묵해요 (휴면)", () => {
    const home = makeHome();
    mkdirSync(join(home, ".axhub", "cache"), { recursive: true });
    const marker = join(home, MARKER_REL);
    writeFileSync(marker, "1.10.28");
    utimesSync(marker, daysAgo(8), daysAgo(8));
    expectSilent(runScript(restartConfirmSh, "", { HOME: home }));
  });

  test("kill switch AXHUB_NO_AUTO_UPDATE → 침묵해요", () => {
    const home = makeHome();
    mkdirSync(join(home, ".axhub", "cache"), { recursive: true });
    writeFileSync(join(home, MARKER_REL), "1.10.28");
    expectSilent(runScript(restartConfirmSh, "", { HOME: home, AXHUB_NO_AUTO_UPDATE: "1" }));
  });

  test("marker kill switch no-auto-update → 침묵해요 (auto-update 계열 공용)", () => {
    const home = makeHomeWithMarker("auto-update");
    mkdirSync(join(home, ".axhub", "cache"), { recursive: true });
    writeFileSync(join(home, MARKER_REL), "1.10.28");
    expectSilent(runScript(restartConfirmSh, "", { HOME: home }));
  });
});

describe("AP-13 Windows 계약 훅 (SessionStart entry 2)", () => {
  test("OS=Windows_NT → 계약을 발행해요", () => {
    expectEmit(runScript(windowsContractSh, "", { OS: "Windows_NT" }), "SessionStart", "Git Bash");
  });

  test("non-Windows(OS 미설정) → 침묵해요", () => {
    expectSilent(runScript(windowsContractSh, ""));
  });

  test("kill switch AXHUB_NO_WINDOWS_CONTRACT → 침묵해요", () => {
    expectSilent(runScript(windowsContractSh, "", { OS: "Windows_NT", AXHUB_NO_WINDOWS_CONTRACT: "1" }));
  });

  test("marker kill switch no-windows-contract → 침묵해요", () => {
    expectSilent(runScript(windowsContractSh, "", { OS: "Windows_NT", HOME: makeHomeWithMarker("windows-contract") }));
  });
});

describe("AP-19 feedback 리포트 계약 훅 (SessionStart entry 5)", () => {
  test("PATH 에 axhub 존재 → 계약을 발행해요", () => {
    const result = runScript(feedbackContractSh, "", { HOME: makeHome(), PATH: SAFE_PATH });
    expectEmit(result, "SessionStart", "axhub feedback -m");
    expect(result.stdout).toContain("예상된 거절은 리포트하지 않아요");
  });

  test("PATH 에 없어도 위치 파일 ~/.axhub/bin-path 가 있으면 발행해요 (AP-17 3-경로)", () => {
    const home = makeHome();
    mkdirSync(join(home, ".axhub"), { recursive: true });
    writeFileSync(join(home, ".axhub", "bin-path"), "/opt/axhub/bin/axhub");
    expectEmit(runScript(feedbackContractSh, "", { HOME: home, PATH: NO_AXHUB_PATH }), "SessionStart", "axhub feedback -m");
  });

  test("canonical ~/.axhub/bin/axhub 만 있어도 발행해요 (AP-17 3-경로)", () => {
    const home = makeHome();
    mkdirSync(join(home, ".axhub", "bin"), { recursive: true });
    writeFileSync(join(home, ".axhub", "bin", "axhub"), "");
    expectEmit(runScript(feedbackContractSh, "", { HOME: home, PATH: NO_AXHUB_PATH }), "SessionStart", "axhub feedback -m");
  });

  test("CLI 가 3-경로 어디에도 없으면 침묵해요", () => {
    expectSilent(runScript(feedbackContractSh, "", { HOME: makeHome(), PATH: NO_AXHUB_PATH }));
  });

  test("kill switch AXHUB_NO_FEEDBACK_REPORT → 침묵해요", () => {
    expectSilent(runScript(feedbackContractSh, "", { HOME: makeHome(), PATH: SAFE_PATH, AXHUB_NO_FEEDBACK_REPORT: "1" }));
  });

  test("marker kill switch no-feedback-report → 침묵해요", () => {
    expectSilent(runScript(feedbackContractSh, "", { HOME: makeHomeWithMarker("feedback-report"), PATH: SAFE_PATH }));
  });
});

describe("AP-14 폴백 훅 (SessionStart entry 3)", () => {
  test("기본 상태에서 update-first 가드를 발행해요", () => {
    expectEmit(runScript(ap14FallbackSh, ""), "SessionStart", "update-first routing guard is active for Code mode");
  });

  test("kill switch AXHUB_NO_UPDATE_ROUTER → 침묵해요", () => {
    expectSilent(runScript(ap14FallbackSh, "", { AXHUB_NO_UPDATE_ROUTER: "1" }));
  });

  test("marker kill switch no-update-router → 침묵해요 (UserPromptSubmit 라우터와 공용)", () => {
    expectSilent(runScript(ap14FallbackSh, "", { HOME: makeHomeWithMarker("update-router") }));
  });
});

// ── hooks.json 합성 command e2e (plugin-root env 치환 경계) ──────────────

// hooks.json 의 SessionStart command 문자열 자체를 `bash -c` 로 실행해요 —
// wrapper 파일 직접 실행(runScript)이 못 보는 `${CLAUDE_PLUGIN_ROOT}` 치환
// 경계를 잠가요. repo 루트를 그대로 쓰면 .git 때문에 entry 1 dev 가드가
// 끼어드니, .git 없는 스테이징 번들 루트(운영 플러그인 캐시 배치와 동형)로
// 각 entry 의 기대 emit/침묵을 그대로 재현해요.
const STAGED_PLUGIN_ROOT = mkdtempSync(join(tmpdir(), "axhub-staged-plugin-"));
cpSync(join(REPO_ROOT, "hooks"), join(STAGED_PLUGIN_ROOT, "hooks"), { recursive: true });

const runSsCommand = (index: number, env: Record<string, string> = {}): RunResult =>
  runInline(ssCommands[index]!, "", { CLAUDE_PLUGIN_ROOT: STAGED_PLUGIN_ROOT, ...env });

describe("SessionStart 합성 command e2e (hooks.json command 문자열 실행)", () => {
  test("entry 1 auto-update: 스테이징 루트 + 캐시 없음 → 운영 emit 이 재현돼요", () => {
    const home = makeHome();
    expectEmit(runSsCommand(0, { HOME: home, PATH: SAFE_PATH }), "SessionStart", "auto-update-prompt.md");
    expect(existsSync(join(home, CACHE_REL))).toBe(true);
  });

  test("entry 1 auto-update: kill switch 침묵도 합성 command 경로에서 재현돼요", () => {
    const home = makeHome();
    expectSilent(runSsCommand(0, { HOME: home, PATH: SAFE_PATH, AXHUB_NO_AUTO_UPDATE: "1" }));
    expect(existsSync(join(home, CACHE_REL))).toBe(false);
  });

  test("entry 2 windows-contract: OS=Windows_NT → 발행, 미설정(non-Windows skip) → 침묵해요", () => {
    expectEmit(runSsCommand(1, { OS: "Windows_NT" }), "SessionStart", "Git Bash");
    expectSilent(runSsCommand(1));
  });

  test("entry 3 update-router-guard: 기본 상태 → update-first 가드를 발행해요", () => {
    expectEmit(runSsCommand(2), "SessionStart", "update-first routing guard is active for Code mode");
  });

  test("entry 4 restart-confirm: fresh marker → confirm prompt 지시를 발행해요", () => {
    const home = makeHome();
    mkdirSync(join(home, ".axhub", "cache"), { recursive: true });
    writeFileSync(join(home, MARKER_REL), "1.10.28");
    expectEmit(runSsCommand(3, { HOME: home }), "SessionStart", "plugin-restart-confirm-prompt.md");
  });

  test("entry 5 feedback-contract: PATH 의 axhub 로 계약을 발행해요", () => {
    expectEmit(runSsCommand(4, { HOME: makeHome(), PATH: SAFE_PATH }), "SessionStart", "axhub feedback -m");
  });
});

describe("CLAUDE_PLUGIN_ROOT 미설정 경계 (omitted-root)", () => {
  test("env 미설정이면 5개 entry 모두 exit 127 + stdout 무JSON 으로 소실돼요 (현행 거동 문서화)", () => {
    // 조용한 소실이 현행 계약이에요 — Claude·Codex 둘 다 env 주입 실측 확인, U1-(p) 재확인 예정.
    for (const command of ssCommands) {
      const result = runInline(command, "");
      expect(result.exitCode).toBe(127);
      expect(result.stdout.trim()).toBe("");
    }
  });
});
