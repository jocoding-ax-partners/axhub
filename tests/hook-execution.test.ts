import { describe, expect, test } from "bun:test";
import { cpSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, statSync, utimesSync, writeFileSync, chmodSync } from "node:fs";
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

// ── AP-26 auto-update worker 하네스 ─────────────────────────────────────
// worker 는 stub axhub/claude 를 PATH 에 두고 전경으로 실행해요 — async 여부는
// harness 몫이라 스크립트는 끝까지 돌고 stdout(알림 JSON)을 돌려줘요. stub 은
// STUB_* env 로 출력·exit 를 정하고 호출 인자를 STUB_CALLS 파일에 남겨요.
const AXHUB_STUB = `#!/usr/bin/env bash
[ -n "$STUB_CALLS" ] && echo "axhub $*" >> "$STUB_CALLS"
case "$*" in
  *--field-expr*) [ -n "$STUB_CHECK_EXIT" ] && exit "$STUB_CHECK_EXIT"; printf '%s\\n' "$STUB_CHECK_OUT" ;;
  "update check --json") printf '%s\\n' "$STUB_CHECK_JSON" ;;
  "update apply"*) exit "\${STUB_APPLY_EXIT:-0}" ;;
esac
exit 0
`;
const CLAUDE_STUB = `#!/usr/bin/env bash
[ -n "$STUB_CALLS" ] && echo "claude $*" >> "$STUB_CALLS"
case "$1 $2" in
  "plugin list") printf '  ❯ axhub@axhub\\n    Version: 1.27.1\\n    Scope: %s\\n    Status: ✔ enabled\\n' "\${STUB_SCOPE:-user}" ;;
  "plugin update") exit "\${STUB_PLUGIN_EXIT:-0}" ;;
esac
exit 0
`;

function makeStubBin(): string {
  const bin = mkdtempSync(join(tmpdir(), "axhub-stub-bin-"));
  for (const [name, body] of [["axhub", AXHUB_STUB], ["claude", CLAUDE_STUB]] as const) {
    writeFileSync(join(bin, name), body);
    chmodSync(join(bin, name), 0o755);
  }
  return bin;
}

function makePluginRoot(version: string): string {
  const root = mkdtempSync(join(tmpdir(), "axhub-plugin-root-"));
  mkdirSync(join(root, ".claude-plugin"), { recursive: true });
  writeFileSync(join(root, ".claude-plugin", "plugin.json"), JSON.stringify({ name: "axhub", version }));
  return root;
}

const STUB_BIN = makeStubBin();
const SAFE_PATH = `${STUB_BIN}:/usr/bin:/bin`;
const NO_AXHUB_PATH = "/usr/bin:/bin";
const PLUGIN_ROOT = makePluginRoot("1.27.1");
const CACHE_REL = join(".axhub", "cache", ".plugin-update-check");
const MARKER_REL = join(".axhub", "cache", ".plugin-update-restart");
const LOG_REL = join(".axhub", "cache", "auto-update.log");
const HALT_REL = join(".axhub", "cache", ".auto-update-halt");
const LOCK_REL = join(".axhub", "cache", ".auto-update.lock");
const BOTH_UPDATES = "v0.38.0 true v0.39.0 false false true 1.28.0";
const CLI_ONLY_UPDATE = "v0.38.0 true v0.39.0 false false false 1.27.1";
const UP_TO_DATE = "v0.38.0 false v0.38.0 false false false 1.27.1";

function daysAgo(days: number): Date {
  return new Date(Date.now() - days * 24 * 60 * 60 * 1000);
}

function minutesAgo(minutes: number): Date {
  return new Date(Date.now() - minutes * 60 * 1000);
}

interface WorkerRun {
  result: RunResult;
  home: string;
  calls: string[];
  log: string;
}

function runWorker(env: Record<string, string>, home = makeHome()): WorkerRun {
  const callsFile = join(home, "stub-calls");
  writeFileSync(callsFile, ""); // 같은 home 재실행 시 이전 run 의 호출 기록이 섞이지 않게 비워요
  const result = runScript(autoUpdateSh, "", {
    HOME: home,
    PATH: SAFE_PATH,
    CLAUDE_PLUGIN_ROOT: PLUGIN_ROOT,
    STUB_CALLS: callsFile,
    ...env,
  });
  const calls = existsSync(callsFile) ? readFileSync(callsFile, "utf8").trim().split("\n").filter(Boolean) : [];
  const log = existsSync(join(home, LOG_REL)) ? readFileSync(join(home, LOG_REL), "utf8") : "";
  return { result, home, calls, log };
}

const applyCalls = (calls: string[]): string[] => calls.filter((line) => line.startsWith("axhub update apply"));
const claudeCalls = (calls: string[]): string[] => calls.filter((line) => line.startsWith("claude "));

describe("auto-update worker (SessionStart entry 1, async — AP-26)", () => {
  test("hooks.json 은 entry 1 에만 async:true + timeout:5 를 달아요 (command 문자열 불변)", () => {
    const entries = hooksFile.hooks.SessionStart.flatMap((group) => group.hooks) as Array<{
      command: string;
      async?: boolean;
      timeout?: number;
    }>;
    expect(entries[0]!.async).toBe(true);
    expect(entries[0]!.timeout).toBe(5);
    for (const entry of entries.slice(1)) {
      expect(entry.async).toBeUndefined();
      expect(entry.timeout).toBeUndefined();
    }
  });

  test("hooks/ 에 에이전트 프롬프트 md 가 없어요 (KD3)", () => {
    const mdFiles = readdirSync(join(REPO_ROOT, "hooks")).filter((name) => name.endsWith(".md"));
    expect(mdFiles).toEqual([]);
  });

  test("캐시 없음 + CLI·플러그인 새 버전 → apply·plugin update 실행, marker·log·알림 JSON (AE1·AE2)", () => {
    const { result, home, calls, log } = runWorker({ STUB_CHECK_OUT: BOTH_UPDATES, STUB_SCOPE: "project" });
    expectEmit(result, "SessionStart", "axhub CLI 가 v0.38.0 → v0.39.0 로 자동 업데이트됐어요");
    expect(result.stdout).toContain("버전 확인 명령은 실행하지 마세요");
    expect(calls[0]).toContain("axhub update check --plugin-version 1.27.1 --field-expr");
    expect(calls.slice(1)).toEqual([
      "axhub update apply --execute --yes --json",
      "claude plugin list",
      "claude plugin marketplace update axhub",
      "claude plugin update axhub@axhub --scope project",
    ]);
    expect(readFileSync(join(home, MARKER_REL), "utf8")).toBe("1.28.0|project");
    expect(log).toContain("host=claude UPDATED cli=v0.38.0->v0.39.0 PLUGIN_UPDATED plugin=1.27.1->1.28.0 scope=project");
    expect(existsSync(join(home, CACHE_REL))).toBe(true);
    expect(existsSync(join(home, LOCK_REL))).toBe(false);
  });

  test("최신 상태 → 침묵, apply·plugin 호출 0, log 는 UP_TO_DATE 한 줄", () => {
    const { result, home, calls, log } = runWorker({ STUB_CHECK_OUT: UP_TO_DATE });
    expectSilent(result);
    expect(calls).toHaveLength(1);
    expect(log.trim().split("\n")).toHaveLength(1);
    expect(log).toContain("UP_TO_DATE cli=v0.38.0 plugin=1.27.1");
    expect(existsSync(join(home, MARKER_REL))).toBe(false);
  });

  test("fresh 캐시(24h 이내) → 즉시 침묵, stub 호출 0", () => {
    const home = makeHome();
    mkdirSync(join(home, ".axhub", "cache"), { recursive: true });
    writeFileSync(join(home, CACHE_REL), "");
    const { result, calls, log } = runWorker({ STUB_CHECK_OUT: BOTH_UPDATES }, home);
    expectSilent(result);
    expect(calls).toEqual([]);
    expect(log).toBe("");
  });

  test("stale 캐시(24h 초과) → 진행하고 캐시 mtime 을 갱신해요", () => {
    const home = makeHome();
    mkdirSync(join(home, ".axhub", "cache"), { recursive: true });
    const cache = join(home, CACHE_REL);
    writeFileSync(cache, "");
    utimesSync(cache, daysAgo(2), daysAgo(2));
    const { result, calls } = runWorker({ STUB_CHECK_OUT: UP_TO_DATE }, home);
    expectSilent(result);
    expect(calls).toHaveLength(1);
    expect(statSync(cache).mtimeMs).toBeGreaterThan(daysAgo(1).getTime());
  });

  test("kill switch AXHUB_NO_AUTO_UPDATE → 침묵 + 캐시·lock·log·marker 전부 없음 (AE4)", () => {
    const { result, home, calls } = runWorker({ STUB_CHECK_OUT: BOTH_UPDATES, AXHUB_NO_AUTO_UPDATE: "1" });
    expectSilent(result);
    expect(calls).toEqual([]);
    for (const rel of [CACHE_REL, LOCK_REL, LOG_REL, MARKER_REL]) {
      expect(existsSync(join(home, rel)), rel).toBe(false);
    }
  });

  test("marker kill switch no-auto-update → 침묵 + 파일 생성 없음", () => {
    const home = makeHomeWithMarker("auto-update");
    const { result, calls } = runWorker({ STUB_CHECK_OUT: BOTH_UPDATES }, home);
    expectSilent(result);
    expect(calls).toEqual([]);
    expect(existsSync(join(home, CACHE_REL))).toBe(false);
  });

  test("axhub 바이너리 부재(3-경로 모두) → 침묵, 캐시 없음", () => {
    const { result, home, calls } = runWorker({ STUB_CHECK_OUT: BOTH_UPDATES, PATH: NO_AXHUB_PATH });
    expectSilent(result);
    expect(calls).toEqual([]);
    expect(existsSync(join(home, CACHE_REL))).toBe(false);
  });

  test("PATH 에 없어도 ~/.axhub/bin-path 위치 파일로 CLI 를 찾아요 (AP-17)", () => {
    const home = makeHome();
    mkdirSync(join(home, ".axhub"), { recursive: true });
    writeFileSync(join(home, ".axhub", "bin-path"), `${join(STUB_BIN, "axhub")}\n`);
    const { result, calls, log } = runWorker({ STUB_CHECK_OUT: UP_TO_DATE, PATH: NO_AXHUB_PATH }, home);
    expectSilent(result);
    expect(calls).toHaveLength(1);
    expect(log).toContain("UP_TO_DATE");
  });

  test("dev 가드 토폴로지 A: <repo>/plugins/axhub in-place 로딩 → 침묵해요", () => {
    const repo = mkdtempSync(join(tmpdir(), "axhub-dev-repo-"));
    mkdirSync(join(repo, ".git"), { recursive: true });
    const pluginRoot = join(repo, "plugins", "axhub");
    mkdirSync(pluginRoot, { recursive: true });
    const { result, home, calls } = runWorker({ STUB_CHECK_OUT: BOTH_UPDATES, CLAUDE_PLUGIN_ROOT: pluginRoot });
    expectSilent(result);
    expect(calls).toEqual([]);
    expect(existsSync(join(home, CACHE_REL))).toBe(false);
  });

  test("dev 가드 토폴로지 B: repo 루트 직접 로딩 (루트 plugin.json 레이아웃) → 침묵해요", () => {
    const repo = mkdtempSync(join(tmpdir(), "axhub-dev-root-"));
    mkdirSync(join(repo, ".git"), { recursive: true });
    const { result, home, calls } = runWorker({ STUB_CHECK_OUT: BOTH_UPDATES, CLAUDE_PLUGIN_ROOT: repo });
    expectSilent(result);
    expect(calls).toEqual([]);
    expect(existsSync(join(home, CACHE_REL))).toBe(false);
  });

  test("disabled=true(패키지 매니저 관리) → apply 미호출, log SKIP_DISABLED", () => {
    const { result, calls, log } = runWorker({ STUB_CHECK_OUT: "v0.38.0 true v0.39.0 true false false 1.27.1" });
    expectSilent(result);
    expect(applyCalls(calls)).toEqual([]);
    expect(log).toContain("SKIP_DISABLED cli=v0.38.0 latest=v0.39.0");
  });

  test("is_downgrade=true(서버 롤백) → apply 미호출, log SKIP_DOWNGRADE", () => {
    const { result, calls, log } = runWorker({ STUB_CHECK_OUT: "v0.38.0 true v0.37.0 false true false 1.27.1" });
    expectSilent(result);
    expect(applyCalls(calls)).toEqual([]);
    expect(log).toContain("SKIP_DOWNGRADE");
  });

  test("apply exit 66(cosign) → halt marker + 보안 알림 1회, 같은 버전은 재시도하지 않아요 (AE3)", () => {
    const first = runWorker({ STUB_CHECK_OUT: CLI_ONLY_UPDATE, STUB_APPLY_EXIT: "66" });
    expectEmit(first.result, "SessionStart", "보안 검증에 실패했어요");
    expect(readFileSync(join(first.home, HALT_REL), "utf8")).toBe("v0.39.0|66");
    expect(first.log).toContain("SECURITY_HALT latest=v0.39.0 exit=66");

    // 다음 주기 — 캐시를 stale 로 돌리면 check 는 다시 하지만 apply 는 건너뛰고 침묵해요.
    utimesSync(join(first.home, CACHE_REL), daysAgo(2), daysAgo(2));
    const second = runWorker({ STUB_CHECK_OUT: CLI_ONLY_UPDATE, STUB_APPLY_EXIT: "66" }, first.home);
    expectSilent(second.result);
    expect(applyCalls(second.calls)).toEqual([]);
    expect(second.log).toContain("SKIP_HALTED latest=v0.39.0");

    // 새 latest 가 나오면 halt 를 지우고 다시 시도해요.
    utimesSync(join(first.home, CACHE_REL), daysAgo(2), daysAgo(2));
    const third = runWorker({ STUB_CHECK_OUT: "v0.38.0 true v0.40.0 false false false 1.27.1" }, first.home);
    expectEmit(third.result, "SessionStart", "v0.38.0 → v0.40.0");
    expect(existsSync(join(first.home, HALT_REL))).toBe(false);
  });

  test("apply 가 그 외 코드로 실패하면 log APPLY_FAILED 만 남기고 침묵해요", () => {
    const { result, log } = runWorker({ STUB_CHECK_OUT: CLI_ONLY_UPDATE, STUB_APPLY_EXIT: "1" });
    expectSilent(result);
    expect(log).toContain("APPLY_FAILED exit=1 latest=v0.39.0");
  });

  test("check 실패(exit 1) → CHECK_FAILED, apply 미호출 — 침묵을 최신으로 읽지 않아요", () => {
    const { result, calls, log } = runWorker({ STUB_CHECK_EXIT: "1" });
    expectSilent(result);
    expect(applyCalls(calls)).toEqual([]);
    expect(log).toContain("CHECK_FAILED rc=1");
  });

  test("필드가 7개 미만이면 CHECK_FAILED 예요", () => {
    const { result, calls, log } = runWorker({ STUB_CHECK_OUT: "v0.38.0 true v0.39.0 false false false" });
    expectSilent(result);
    expect(applyCalls(calls)).toEqual([]);
    expect(log).toContain("CHECK_FAILED");
  });

  test("구 CLI(--field-expr 없음, exit 64) → --json fallback 으로 CLI 만 apply 하고 플러그인은 미뤄요", () => {
    const { result, calls, log } = runWorker({
      STUB_CHECK_EXIT: "64",
      STUB_CHECK_JSON:
        '{"current":"v0.30.0","disabled":false,"has_update":true,"is_downgrade":false,"latest":"v0.39.0","plugin":{"current":"1.27.1","has_update":true,"latest":"1.28.0"}}',
    });
    expectEmit(result, "SessionStart", "v0.30.0 → v0.39.0");
    expect(calls[1]).toBe("axhub update check --json");
    expect(applyCalls(calls)).toHaveLength(1);
    expect(claudeCalls(calls)).toEqual([]);
    expect(log).toContain("UPDATED cli=v0.30.0->v0.39.0");
  });

  test("lock 이 살아 있으면(TTL 미만) 침묵 + 캐시 미touch + 호출 0 (AE5)", () => {
    const home = makeHome();
    mkdirSync(join(home, LOCK_REL), { recursive: true });
    const { result, calls } = runWorker({ STUB_CHECK_OUT: BOTH_UPDATES }, home);
    expectSilent(result);
    expect(calls).toEqual([]);
    expect(existsSync(join(home, CACHE_REL))).toBe(false);
    expect(existsSync(join(home, LOCK_REL))).toBe(true);
  });

  test("lock TTL(30분) 초과 → 회수하고 진행해요", () => {
    const home = makeHome();
    mkdirSync(join(home, LOCK_REL), { recursive: true });
    utimesSync(join(home, LOCK_REL), minutesAgo(45), minutesAgo(45));
    const { result, calls } = runWorker({ STUB_CHECK_OUT: UP_TO_DATE }, home);
    expectSilent(result);
    expect(calls).toHaveLength(1);
    expect(existsSync(join(home, LOCK_REL))).toBe(false);
  });

  test("plugin update 실패 → PLUGIN_FAILED, restart marker 없음", () => {
    const { result, home, log } = runWorker({ STUB_CHECK_OUT: "v0.38.0 false v0.38.0 false false true 1.28.0", STUB_PLUGIN_EXIT: "1" });
    expectSilent(result);
    expect(log).toContain("PLUGIN_FAILED plugin=1.27.1 latest=1.28.0 scope=user");
    expect(existsSync(join(home, MARKER_REL))).toBe(false);
  });

  test("host CLI(claude) 가 없으면 플러그인만 건너뛰어요", () => {
    const bin = mkdtempSync(join(tmpdir(), "axhub-only-bin-"));
    cpSync(join(STUB_BIN, "axhub"), join(bin, "axhub"));
    const { result, log } = runWorker({ STUB_CHECK_OUT: "v0.38.0 false v0.38.0 false false true 1.28.0", PATH: `${bin}:/usr/bin:/bin` });
    expectSilent(result);
    expect(log).toContain("PLUGIN_SKIPPED reason=host_cli_missing latest=1.28.0");
  });

  test("log 가 200줄을 넘으면 앞을 잘라요", () => {
    const home = makeHome();
    mkdirSync(join(home, ".axhub", "cache"), { recursive: true });
    writeFileSync(join(home, LOG_REL), Array.from({ length: 205 }, (_, i) => `old line ${i}`).join("\n") + "\n");
    const { log } = runWorker({ STUB_CHECK_OUT: UP_TO_DATE }, home);
    const lines = log.trim().split("\n");
    expect(lines).toHaveLength(200);
    expect(lines[lines.length - 1]).toContain("UP_TO_DATE");
  });
});

describe("restart-confirm 훅 (SessionStart entry 4 — AP-26)", () => {
  function homeWithMarker(content: string, ageDays = 0): string {
    const home = makeHome();
    mkdirSync(join(home, ".axhub", "cache"), { recursive: true });
    const marker = join(home, MARKER_REL);
    writeFileSync(marker, content);
    if (ageDays > 0) utimesSync(marker, daysAgo(ageDays), daysAgo(ageDays));
    return home;
  }

  test("로드 버전 ≥ marker → 적용 확인 emit + marker 삭제 (AE2 후반)", () => {
    const home = homeWithMarker("1.28.0|user");
    const result = runScript(restartConfirmSh, "", { HOME: home, CLAUDE_PLUGIN_ROOT: makePluginRoot("1.28.0") });
    expectEmit(result, "SessionStart", "플러그인 v1.28.0 적용을 확인했어요");
    expect(existsSync(join(home, MARKER_REL))).toBe(false);
  });

  test("로드 버전 < marker → 재시작 안내 emit + marker 유지 (AE2 전반)", () => {
    const home = homeWithMarker("1.28.0|user");
    const result = runScript(restartConfirmSh, "", { HOME: home, CLAUDE_PLUGIN_ROOT: makePluginRoot("1.27.1") });
    expectEmit(result, "SessionStart", "Claude Code 를 재시작하면 적용된다고");
    expect(result.stdout).toContain("v1.28.0");
    expect(existsSync(join(home, MARKER_REL))).toBe(true);
  });

  test("scope 없는 구 marker 도 버전만 읽고, semver 는 자리별 숫자로 비교해요 (1.30.0 ≥ 1.28.0)", () => {
    const home = homeWithMarker("1.28.0");
    const result = runScript(restartConfirmSh, "", { HOME: home, CLAUDE_PLUGIN_ROOT: makePluginRoot("1.30.0") });
    expectEmit(result, "SessionStart", "플러그인 v1.30.0 적용을 확인했어요");
    expect(existsSync(join(home, MARKER_REL))).toBe(false);
  });

  test("plugin.json 을 못 읽으면 판정 불가로 침묵하고 marker 를 둬요", () => {
    const home = homeWithMarker("1.28.0|user");
    expectSilent(runScript(restartConfirmSh, "", { HOME: home }));
    expect(existsSync(join(home, MARKER_REL))).toBe(true);
  });

  test("marker 없음 → 침묵해요", () => {
    expectSilent(runScript(restartConfirmSh, "", { HOME: makeHome(), CLAUDE_PLUGIN_ROOT: makePluginRoot("1.28.0") }));
  });

  test("TTL(7일) 초과 marker → 침묵해요 (휴면)", () => {
    const home = homeWithMarker("1.28.0|user", 8);
    expectSilent(runScript(restartConfirmSh, "", { HOME: home, CLAUDE_PLUGIN_ROOT: makePluginRoot("1.28.0") }));
    expect(existsSync(join(home, MARKER_REL))).toBe(true);
  });

  test("kill switch AXHUB_NO_AUTO_UPDATE → 침묵해요", () => {
    const home = homeWithMarker("1.28.0|user");
    expectSilent(runScript(restartConfirmSh, "", { HOME: home, CLAUDE_PLUGIN_ROOT: makePluginRoot("1.28.0"), AXHUB_NO_AUTO_UPDATE: "1" }));
  });

  test("marker kill switch no-auto-update → 침묵해요 (auto-update 계열 공용)", () => {
    const home = makeHomeWithMarker("auto-update");
    mkdirSync(join(home, ".axhub", "cache"), { recursive: true });
    writeFileSync(join(home, MARKER_REL), "1.28.0|user");
    expectSilent(runScript(restartConfirmSh, "", { HOME: home, CLAUDE_PLUGIN_ROOT: makePluginRoot("1.28.0") }));
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
cpSync(join(REPO_ROOT, ".claude-plugin"), join(STAGED_PLUGIN_ROOT, ".claude-plugin"), { recursive: true });

const runSsCommand = (index: number, env: Record<string, string> = {}): RunResult =>
  runInline(ssCommands[index]!, "", { CLAUDE_PLUGIN_ROOT: STAGED_PLUGIN_ROOT, ...env });

describe("SessionStart 합성 command e2e (hooks.json command 문자열 실행)", () => {
  test("entry 1 auto-update: 스테이징 루트 + 캐시 없음 + 새 버전 → 알림 emit 이 재현돼요 (AP-26)", () => {
    const home = makeHome();
    expectEmit(
      runSsCommand(0, { HOME: home, PATH: SAFE_PATH, STUB_CHECK_OUT: BOTH_UPDATES }),
      "SessionStart",
      "자동 업데이트됐어요",
    );
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

  test("entry 4 restart-confirm: fresh marker + 로드 버전이 더 높음 → 적용 확인을 발행해요 (AP-26)", () => {
    const home = makeHome();
    mkdirSync(join(home, ".axhub", "cache"), { recursive: true });
    writeFileSync(join(home, MARKER_REL), "1.10.28");
    expectEmit(runSsCommand(3, { HOME: home }), "SessionStart", "적용을 확인했어요");
    expect(existsSync(join(home, MARKER_REL))).toBe(false);
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
