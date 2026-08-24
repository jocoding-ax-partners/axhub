import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, extname, join, relative, resolve } from "node:path";

const REPO_ROOT = resolve(join(import.meta.dir, ".."));
const DEFAULT_OUT_DIR = join(REPO_ROOT, "dist", "axhub-plugin");
const DEFAULT_CODEX_OUT_DIR = join(REPO_ROOT, "dist", "axhub-plugin-codex");
const CODEX_OVERRIDES_DIR = join(REPO_ROOT, "codex-overrides");
const ROOT_FILES = ["README.md", "LICENSE", "POLICY.md"] as const;
const ROOT_DIRS = [".claude-plugin", "skills", "hooks"] as const;
const DENY_NAMES = new Set([
  ".DS_Store",
  ".axhub",
  ".axhub-state",
  ".autoplan",
  ".bun",
  ".cache",
  ".cargo",
  ".claude",
  ".codegraph",
  ".git",
  ".github",
  ".gitnexus",
  ".gjc",
  ".graphify_python",
  ".obsidian",
  ".omc",
  ".omx",
  ".ouroboros",
  ".plan",
  ".qa-live",
  ".serena",
  ".specify",
  ".understand-anything",
  "CHANGELOG.md",
  "CLAUDE.md",
  "AGENTS.md",
  "node_modules",
  "graphify-out",
  "tests",
  "scripts",
  "dist",
  "test-results.json",
]);

type Host = "claude" | "codex";

interface Options {
  outDir: string;
  json: boolean;
  host: Host;
}

interface BundleStats {
  outDir: string;
  files: number;
  bytes: number;
  host: Host;
}

// ── codex 파생 상수 (KTD1·KTD5·KTD7·KTD11, U5) ──────────────────────────────
// 치환은 longest-first 가 계약이에요 — 병기 구문 전체를 먼저 소비해 이중 적용을
// 막아요 (tests/codex-bundle.test.ts 가 정렬을 assert 해요).
export const CODEX_SUBSTITUTIONS: ReadonlyArray<readonly [from: string, to: string]> = [
  [
    "```typescript\nTodoWrite({ todos: [\n  { content: \"테이블 생성\",   status: \"in_progress\", activeForm: \"테이블 만드는 중\" },\n  { content: \"환경변수 추가\", status: \"pending\",     activeForm: \"env 추가하는 중\" }\n]})\n```",
    "참고 항목: `테이블 생성`(진행 중) → `환경변수 추가`(대기). 인자 shape 은 host 가 노출한 도구 스키마를 그대로 따라요.",
  ],
  [
    "Markdown 링크(`[https://...](github.com/...)` 포함), `Monitor`, `ScheduleWakeup`, `TaskOutput`, `읽는 중 <output>`, 임시 출력 파일 읽기 카드로 코드 노출 금지.",
    "Markdown 링크(`[https://...](github.com/...)` 포함), 백그라운드 감시, `읽는 중 <output>`, 임시 출력 파일 읽기 카드로 코드 노출 금지.",
  ],
  [
    "Never poll deployment status with a shell loop, Claude Desktop Monitor, ScheduleWakeup, or any background watcher.",
    "Never poll deployment status with a shell loop or any background watcher.",
  ],
  [
    "Do not use Monitor, ScheduleWakeup, TaskOutput, or background output file reads as the resume control plane.",
    "Do not use background watchers or output-file reads as the resume control plane.",
  ],
  [
    "Do not use Claude Desktop Monitor/ScheduleWakeup/background-task output as the device-flow control plane.",
    "Do not use background-task output as the device-flow control plane.",
  ],
  [
    "네이티브 선택 UI 가 있으면 그걸로 묻고, 없으면 같은 확인을 명시 텍스트 승인 1회로 받고, 둘 다 불가한 headless 에서는 실행 없이 멈춰요 — 승인을 조용히 건너뛰지 않아요.",
    "같은 확인을 명시 텍스트 승인 1회로 받아요 — 유효한 승인은 preview 를 본 뒤 사용자가 새로 입력한 canonical 승인 문구뿐이고, 요청과 함께 미리 넣어 둔 문구·유사 표현·무응답은 승인이 아니에요(그 경우 preview 를 보여주고 새 승인을 기다려요). 승인 채널이 없는 headless 에서는 실행 없이 멈춰요 — 승인을 조용히 건너뛰지 않아요.",
  ],
  [
    "`CI` 가 있음, `CLAUDE_NON_INTERACTIVE` 가 있음, `claude -p`·`codex exec` 같은 subprocess/headless 호출임",
    "`CI` 가 있음, `CODEX_NON_INTERACTIVE` 가 있음, `codex exec` 같은 subprocess/headless 호출임",
  ],
  [
    "not from `Monitor`, `ScheduleWakeup`, `TaskOutput`, or reading a generated output file",
    "not from a background watcher or a generated output file",
  ],
  [
    "Claude Desktop 에서는 Monitor, background task, ScheduleWakeup, output 파일 읽기,",
    "Codex 에서는 background task, output 파일 읽기,",
  ],
  [
    "Prefer separate short tool calls or an actual ScheduleWakeup.",
    "Prefer separate short tool calls.",
  ],
  ["`claude -p`·`codex exec`, CI, `$CLAUDE_NON_INTERACTIVE`, ", "`codex exec`, CI, "],
  ["(`claude -p` / CI / `$CLAUDE_NON_INTERACTIVE` / no TTY)", "(`codex exec` / CI / no TTY)"],
  ["`ScheduleWakeup`, 내부 task 이름, 브라우저/preview 도구 이름을", "내부 task 이름, 브라우저/preview 도구 이름을"],
  ["`claude -p`·CI·`$CLAUDE_NON_INTERACTIVE`·TTY 없음", "`codex exec`·CI·TTY 없음"],
  ["`Monitor`, `ScheduleWakeup`, background watch 와", "백그라운드 감시와"],
  ["`실행됨 명령 N개`, `TaskOutput 사용함`, tool 카드,", "`실행됨 명령 N개`, tool 카드,"],
  [
    "## Reference Loading\n\n이 top-level 파일은",
    "## Codex 첫 세션 안내\n\n- 첫 axhub 명령에서 네트워크 접근 승인을 한 번 물어요 — 허용해야 axhub 백엔드에 닿아요.\n- 시작 시 훅 신뢰를 묻는데, 신뢰하지 않으면 자동 업데이트·라우팅 가드가 조용히 꺼져요. `/hooks` 에서 언제든 다시 켤 수 있어요.\n- 선택 카드로 답하고 싶으면 `~/.codex/config.toml` 의 `[features]` 에 `default_mode_request_user_input = true` 한 줄을 더하면 돼요. 켠 경우 빈 답변은 미승인으로 처리돼요. 설정을 대신 바꾸지는 않아요.\n\n## Reference Loading\n\n이 top-level 파일은",
  ],
  ["[ -n \"${CLAUDE_NON_INTERACTIVE:-}\" ]", "[ -n \"${CODEX_NON_INTERACTIVE:-}\" ]"],
  ["claude plugin marketplace update", "codex plugin marketplace upgrade"],
  ["`Monitor 사용` 권한 카드가 뜨는 명령은 실패예요.", "백그라운드 감시 권한 카드가 뜨는 명령은 실패예요."],
  [
    "## 라우팅 경계\n\n- `import`: 현재 폴더가",
    "## 승인 게이트 계약 (요약)\n\ncodex 는 이 본문을 파일 앞에서부터 8,000B 만 읽어요. 아래 게이트는 뒤쪽 절차 설명이 잘려도 그대로 지켜요.\n\n1. 가져오기 승인 — import preview 를 보여준 뒤 `이 앱을 axhub에 가져와서 미리보기대로 진행할까요?` 로 한 번만 묻고, 이 승인 하나가 axhub 진입 확인을 겸해요. 승인 전에는 `--execute` 를 실행하지 않아요.\n2. 승인 방식 — 같은 확인을 명시 텍스트 승인 1회로 받아요. preview 를 본 뒤 사용자가 새로 입력한 canonical 승인 문구만 유효하고, 미리 넣어 둔 문구·유사 표현·무응답은 승인이 아니에요. 승인 채널이 없는 headless 에서는 실행 없이 멈춰요 — 승인을 조용히 건너뛰지 않아요.\n3. 빈 답변 = 미승인 — 선택 카드로 물은 경우 빈 답변이 돌아오면 미승인이에요 — 카드가 자동 해제된 것이므로 실행하지 않고 다시 물어요. 카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요.\n\n**질문 방식.** 선택지를 번호 메뉴로 출력하지 않아요 — 한 문장 확인형으로 묻고, 추천안을 먼저 두고 `(추천)` 을 붙여요. 질문 메시지 안에 답→행동 매핑을 같이 써요(예: `진행` 이면 시작하고 `취소` 면 여기서 멈춰요). 질문한 턴은 도구 호출 없이 끝내고 답을 기다려요. 비파괴 선택은 숫자·서수·라벨·앞글자 어느 쪽으로 답해도 알아듣고, 파괴 게이트만 canonical 문구를 그대로 받아요.\n\n## 라우팅 경계\n\n- `import`: 현재 폴더가",
  ],
  [
    "## Scope\n\n빈 디렉토리 axhub 템플릿 앱",
    "## 승인 게이트 계약 (요약)\n\ncodex 는 이 본문을 파일 앞에서부터 8,000B 만 읽어요. 아래 네 게이트는 뒤쪽 절차 설명이 잘려도 그대로 지켜요.\n\n1. 템플릿 선택 — `어떤 템플릿으로 시작할까요?` 로 묻고, backend 가 실제로 가진 template 만 보여줘요.\n2. 앱 이름 확인 — `앱 이름을 무엇으로 할까요?` 로 묻고, 답을 받은 뒤에만 `--name`/`--slug` 를 확정해요.\n3. 생성·배포 승인 — dry-run preview 를 보여준 뒤 `지금 만들고 배포까지 진행할까요?` 로 묻고, 승인을 받은 뒤에만 `--execute` 를 실행해요.\n4. 승인 방식 — 같은 확인을 명시 텍스트 승인 1회로 받아요. preview 를 본 뒤 사용자가 새로 입력한 canonical 승인 문구만 유효하고, 미리 넣어 둔 문구·유사 표현·무응답은 승인이 아니에요. 승인 채널이 없는 headless 에서는 실행 없이 멈춰요 — 승인을 조용히 건너뛰지 않아요.\n5. 빈 답변 = 미승인 — 선택 카드로 물은 경우 빈 답변이 돌아오면 미승인이에요 — 카드가 자동 해제된 것이므로 실행하지 않고 다시 물어요. 카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요.\n\n**질문 방식.** 선택지를 번호 메뉴로 출력하지 않아요 — 한 문장 확인형으로 묻고, 추천안을 먼저 두고 `(추천)` 을 붙여요. 질문 메시지 안에 답→행동 매핑을 같이 써요(예: `진행` 이면 시작하고 `취소` 면 여기서 멈춰요). 질문한 턴은 도구 호출 없이 끝내고 답을 기다려요. 비파괴 선택은 숫자·서수·라벨·앞글자 어느 쪽으로 답해도 알아듣고, 파괴 게이트만 canonical 문구를 그대로 받아요.\n\n## Scope\n\n빈 디렉토리 axhub 템플릿 앱",
  ],
  [
    "## First Visible Sentence",
    "**질문 방식.** 선택지를 번호 메뉴로 출력하지 않아요 — 한 문장 확인형으로 묻고, 추천안을 먼저 두고 `(추천)` 을 붙여요. 질문 메시지 안에 답→행동 매핑을 같이 써요(예: `진행` 이면 시작하고 `취소` 면 여기서 멈춰요). 질문한 턴은 도구 호출 없이 끝내고 답을 기다려요. 비파괴 선택은 숫자·서수·라벨·앞글자 어느 쪽으로 답해도 알아듣고, 파괴 게이트만 canonical 문구를 그대로 받아요.\n\n**장기 대기.** codex 는 긴 명령을 최대 30초에 yield 하고 백그라운드 터미널로 넘겨요 — yield 는 실패도 완료도 아니에요. `deploy verify --wait` 가 yield 되면 같은 명령을 다시 실행하지 말고 같은 터미널을 빈 입력으로 폴링해 완주를 기다려요. 성공 선언 규칙은 그대로예요.\n\n## 승인 게이트 계약 (요약)\n\ncodex 는 이 본문을 파일 앞에서부터 8,000B 만 읽어요. 승인 방식은 명시 텍스트 승인 1회예요 — preview 를 본 뒤 사용자가 새로 입력한 canonical 승인 문구만 유효하고, 미리 넣어 둔 문구·유사 표현·무응답은 승인이 아니에요. 승인 채널이 없는 headless 에서는 실행 없이 멈춰요 — 승인을 조용히 건너뛰지 않아요. 선택 카드로 물은 경우 빈 답변이 돌아오면 미승인이에요 — 카드가 자동 해제된 것이므로 실행하지 않고 다시 물어요. 카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요.\n\n## First Visible Sentence",
  ],
  ["`claude -p`·`codex exec`", "`codex exec`"],
  ["Claude Desktop Code 모드", "Codex"],
  [".plugin-update-restart", ".plugin-update-restart-codex"],
  ["claude plugin update", "codex plugin marketplace upgrade"],
  [".plugin-update-check", ".plugin-update-check-codex"],
  ["claude plugin list", "codex plugin list"],
  ["command -v claude", "command -v codex"],
  ["AskUserQuestion", "명시 텍스트 승인"],
  ["claude mcp add", "codex mcp add"],
  ["Claude Desktop", "Codex"],
  ["Claude Code", "Codex"],
  ["claude -p", "codex exec"],
  ["TodoWrite", "update_plan"],
  ["Claude", "Codex"],
  [
    "## 순서",
    "**질문 방식.** 선택지를 번호 메뉴로 출력하지 않아요 — 한 문장 확인형으로 묻고, 추천안을 먼저 두고 `(추천)` 을 붙여요. 질문 메시지 안에 답→행동 매핑을 같이 써요(예: `진행` 이면 시작하고 `취소` 면 여기서 멈춰요). 질문한 턴은 도구 호출 없이 끝내고 답을 기다려요. 비파괴 선택은 숫자·서수·라벨·앞글자 어느 쪽으로 답해도 알아듣고, 파괴 게이트만 canonical 문구를 그대로 받아요.\n\n## 승인 게이트 계약 (요약)\n\ncodex 는 이 본문을 파일 앞에서부터 8,000B 만 읽어요. 승인 방식은 명시 텍스트 승인 1회예요 — preview 를 본 뒤 사용자가 새로 입력한 canonical 승인 문구만 유효하고, 미리 넣어 둔 문구·유사 표현·무응답은 승인이 아니에요. 승인 채널이 없는 headless 에서는 실행 없이 멈춰요 — 승인을 조용히 건너뛰지 않아요. 선택 카드로 물은 경우 빈 답변이 돌아오면 미승인이에요 — 카드가 자동 해제된 것이므로 실행하지 않고 다시 물어요. 카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요.\n\n## 순서",
  ],
  ["AUQ", "명시 텍스트 승인"],
];

const CODEX_TEXT_EXTENSIONS = new Set([".md", ".sh", ".json"]);
// AP-14 fallback + AP-19 는 합본 wrapper 하나로 대체돼요 (trust 표면 축소).
const CODEX_SUPERSEDED_WRAPPERS = new Set([
  "session-update-router-guard.sh",
  "session-feedback-contract.sh",
]);
const CODEX_MERGED_WRAPPER = "session-always-on-codex.sh";
// codex-overrides 에서 번들 경로로 mirror-copy 하지 않는 예약 자산이에요.
const CODEX_OVERRIDE_RESERVED = new Set(["routing", "SOURCE_HASHES.json"]);
const CODEX_OVERRIDE_RESERVED_HOOK_DIRS = new Set(["context"]);
const CODEX_PLUGIN_NAME = "axhub-codex";
const CODEX_RECOVERY_LINE =
  "> 이 본문이 중간에 끊겨 보이면 설치 경로의 이 SKILL.md 원문을 열어 전체 절차를 확인한 뒤 진행해요.";

const parseArgs = (): Options => {
  let outDir: string | undefined;
  let json = false;
  let host: Host = "claude";
  const args = Bun.argv.slice(2);
  for (let i = 0; i < args.length; i += 1) {
    const arg = args[i];
    if (arg === "--json") {
      json = true;
    } else if (arg === "--out") {
      const value = args[i + 1];
      if (!value) throw new Error("--out requires a path");
      outDir = resolve(value);
      i += 1;
    } else if (arg === "--host") {
      const value = args[i + 1];
      if (value !== "claude" && value !== "codex") throw new Error("--host requires claude|codex");
      host = value;
      i += 1;
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }
  return { outDir: outDir ?? (host === "codex" ? DEFAULT_CODEX_OUT_DIR : DEFAULT_OUT_DIR), json, host };
};

const assertSafeOutDir = (outDir: string): void => {
  const normalized = resolve(outDir);
  if (normalized === REPO_ROOT || normalized === dirname(REPO_ROOT) || normalized === "/") {
    throw new Error(`refusing to clear unsafe output path: ${outDir}`);
  }
};

const isDenied = (path: string): boolean => {
  const parts = relative(REPO_ROOT, path).split("/");
  return parts.some((part) => DENY_NAMES.has(part));
};

const copyTree = (src: string, dest: string): void => {
  if (isDenied(src)) return;
  const stat = statSync(src);
  if (stat.isDirectory()) {
    mkdirSync(dest, { recursive: true });
    for (const entry of readdirSync(src)) {
      copyTree(join(src, entry), join(dest, entry));
    }
    return;
  }
  if (!stat.isFile()) return;
  mkdirSync(dirname(dest), { recursive: true });
  copyFileSync(src, dest);
};

const collectStats = (dir: string): { files: number; bytes: number } => {
  let files = 0;
  let bytes = 0;
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    const stat = statSync(path);
    if (stat.isDirectory()) {
      const child = collectStats(path);
      files += child.files;
      bytes += child.bytes;
    } else if (stat.isFile()) {
      files += 1;
      bytes += stat.size;
    }
  }
  return { files, bytes };
};

const walkFiles = (dir: string): string[] => {
  const files: string[] = [];
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    const stat = statSync(path);
    if (stat.isDirectory()) files.push(...walkFiles(path));
    else if (stat.isFile()) files.push(path);
  }
  return files;
};

// ── codex 파생 단계들 ────────────────────────────────────────────────────────

// codex-overrides 의 mirror 경로 파일을 번들 위에 스왑해요 (routing/·hooks/context/
// ·SOURCE_HASHES.json 은 transform 입력 자산이라 mirror 대상이 아니에요).
const applyCodexOverrides = (outDir: string): void => {
  if (!existsSync(CODEX_OVERRIDES_DIR)) return;
  for (const entry of readdirSync(CODEX_OVERRIDES_DIR)) {
    if (CODEX_OVERRIDE_RESERVED.has(entry)) continue;
    const src = join(CODEX_OVERRIDES_DIR, entry);
    if (entry === "hooks" && statSync(src).isDirectory()) {
      for (const hookEntry of readdirSync(src)) {
        if (CODEX_OVERRIDE_RESERVED_HOOK_DIRS.has(hookEntry)) continue;
        copyTree(join(src, hookEntry), join(outDir, "hooks", hookEntry));
      }
      continue;
    }
    copyTree(src, join(outDir, entry));
  }
};

const applyCodexSubstitutions = (outDir: string): void => {
  for (const file of walkFiles(outDir)) {
    if (!CODEX_TEXT_EXTENSIONS.has(extname(file))) continue;
    const original = readFileSync(file, "utf8");
    let next = original;
    for (const [from, to] of CODEX_SUBSTITUTIONS) {
      next = next.replaceAll(from, to);
    }
    if (next !== original) writeFileSync(file, next);
  }
};

const shellQuote = (value: string): string => `'${value.replaceAll("'", `'\\''`)}'`;

const suppressedSessionStartJson = (additionalContext: string): string =>
  JSON.stringify({
    continue: true,
    suppressOutput: true,
    hookSpecificOutput: { hookEventName: "SessionStart", additionalContext },
  });

const readOverrideContext = (name: string): string => {
  const path = join(CODEX_OVERRIDES_DIR, "hooks", "context", name);
  const text = readFileSync(path, "utf8").trim();
  if (!text) throw new Error(`empty codex override context: ${relative(REPO_ROOT, path)}`);
  return text;
};

// 소스 wrapper 의 printf JSON 에서 additionalContext 원문을 추출해요 — 문안의
// 소유자는 소스 스크립트라 transform 이 하드코딩하지 않아요.
const extractAdditionalContext = (scriptSource: string, scriptName: string): string => {
  const match = scriptSource.match(/\{"continue":true[\s\S]*?\}\}/);
  if (!match) throw new Error(`cannot find hook JSON in ${scriptName}`);
  const parsed = JSON.parse(match[0]) as {
    hookSpecificOutput?: { additionalContext?: string };
  };
  const context = parsed.hookSpecificOutput?.additionalContext;
  if (!context) throw new Error(`missing additionalContext in ${scriptName}`);
  return context;
};

interface CodexHookEntry {
  type: string;
  command: string;
  commandWindows?: string;
  [key: string]: unknown;
}

interface HooksJson {
  hooks: Record<string, Array<{ matcher?: string; hooks: CodexHookEntry[] }>>;
}

const codexCommandWindows = (scriptName: string): string =>
  `where bash >nul 2>nul && bash "\${CLAUDE_PLUGIN_ROOT}/hooks/${scriptName}" || cd .`;

const wrapperScriptName = (command: string): string => {
  const match = command.match(/hooks\/([A-Za-z0-9._-]+\.sh)/);
  if (!match) throw new Error(`cannot derive wrapper script from command: ${command}`);
  return match[1];
};

const transformCodexHooks = (outDir: string): void => {
  const hooksDir = join(outDir, "hooks");

  // 1) 합본 wrapper 생성 — AP-14 fallback 문안은 override 가, AP-19 문안은 소스
  //    wrapper 가 소유해요. kill switch 2계층(env·marker)과 AP-17 CLI 존재 게이트를
  //    분기별로 보존하고 JSON 은 한 번만 emit 해요.
  const feedbackSource = readFileSync(join(hooksDir, "session-feedback-contract.sh"), "utf8");
  const feedbackContext = extractAdditionalContext(feedbackSource, "session-feedback-contract.sh");
  const routerContext = readOverrideContext("update-first.md");
  const questionContext = readOverrideContext("question-protocol.md");
  // 3 블록 × 각자의 kill switch → 비어 있지 않은 조합마다 JSON 을 미리 만들어
  // if/elif 로 emit 해요. bash 안에서 JSON 을 조립하지 않아 이스케이프 사고가 없어요.
  const blocks = [
    { flag: "R", context: routerContext },
    { flag: "Q", context: questionContext },
    { flag: "F", context: feedbackContext },
  ] as const;
  const branches: string[] = [];
  for (let mask = (1 << blocks.length) - 1; mask >= 1; mask -= 1) {
    const on = blocks.filter((_, idx) => (mask >> (blocks.length - 1 - idx)) & 1);
    const cond = on.map((b) => `[ "$${b.flag}" = 1 ]`).join(" && ");
    const off = blocks.filter((b) => !on.includes(b)).map((b) => `[ "$${b.flag}" = 0 ]`);
    const guard = [cond, ...off].join(" && ");
    const json = suppressedSessionStartJson(on.map((b) => b.context).join("\n\n"));
    branches.push(
      `${branches.length === 0 ? "if" : "elif"} ${guard}; then printf '%s\\n' ${shellQuote(json)}`,
    );
  }
  const mergedScript = [
    "#!/usr/bin/env bash",
    "# codex SessionStart 합본 훅이에요 — AP-14 update-first fallback + 질문 프로토콜 + AP-19 실패 리포트.",
    "# transform 이 생성해요. 문안은 codex-overrides/hooks/context/ 와 소스 wrapper 가 소유해요.",
    "R=1; Q=1; F=1",
    '[ -n "$AXHUB_NO_UPDATE_ROUTER" ] && R=0; [ -f "$HOME/.axhub/config/no-update-router" ] && R=0',
    '[ -n "$AXHUB_NO_QUESTION_PROTOCOL" ] && Q=0; [ -f "$HOME/.axhub/config/no-question-protocol" ] && Q=0',
    '[ -n "$AXHUB_NO_FEEDBACK_REPORT" ] && F=0; [ -f "$HOME/.axhub/config/no-feedback-report" ] && F=0',
    'if [ "$F" = 1 ]; then command -v axhub >/dev/null 2>&1 || [ -f "$HOME/.axhub/bin-path" ] || [ -f "$HOME/.axhub/bin/axhub" ] || [ -f "$HOME/.axhub/bin/axhub.exe" ] || F=0; fi',
    ...branches,
    "fi",
    "",
  ].join("\n");
  writeFileSync(join(hooksDir, CODEX_MERGED_WRAPPER), mergedScript);

  // 2) 대체된 wrapper 2개 제거.
  for (const name of CODEX_SUPERSEDED_WRAPPERS) {
    rmSync(join(hooksDir, name), { force: true });
  }

  // 3) update-router.sh(UserPromptSubmit) 의 컨텍스트를 codex 문안으로 재작성해요 —
  //    게이트 파이프라인(kill switch → prompt 키 → axhub 토큰 → 키워드)은 그대로예요.
  const routerPath = join(hooksDir, "update-router.sh");
  const routerSource = readFileSync(routerPath, "utf8");
  const routerJsonMatch = routerSource.match(/^\{"continue":true.*\}\}$/m);
  if (!routerJsonMatch) throw new Error("cannot find update-router.sh hook JSON line");
  const routerOverride = readOverrideContext("update-router.md");
  const rewrittenRouter = routerSource.replace(
    routerJsonMatch[0],
    JSON.stringify({
      continue: true,
      suppressOutput: true,
      hookSpecificOutput: { hookEventName: "UserPromptSubmit", additionalContext: routerOverride },
    }),
  );
  writeFileSync(routerPath, rewrittenRouter);

  // 4) hooks.json 재작성 — shell 키 제거(U1-(c): 런타임 무시), commandWindows 추가,
  //    대체된 entry 를 합본 entry 로 교체해요.
  const hooksJsonPath = join(hooksDir, "hooks.json");
  const hooksJson = JSON.parse(readFileSync(hooksJsonPath, "utf8")) as HooksJson;
  for (const eventEntries of Object.values(hooksJson.hooks)) {
    for (const entry of eventEntries) {
      const nextHooks: CodexHookEntry[] = [];
      for (const hook of entry.hooks) {
        const scriptName = wrapperScriptName(hook.command);
        if (CODEX_SUPERSEDED_WRAPPERS.has(scriptName)) {
          if (scriptName === "session-update-router-guard.sh") {
            nextHooks.push({
              type: "command",
              command: `bash "\${CLAUDE_PLUGIN_ROOT}/hooks/${CODEX_MERGED_WRAPPER}"`,
              commandWindows: codexCommandWindows(CODEX_MERGED_WRAPPER),
            });
          }
          continue;
        }
        const { shell: _shell, ...rest } = hook as CodexHookEntry & { shell?: string };
        nextHooks.push({ ...rest, commandWindows: codexCommandWindows(scriptName) });
      }
      entry.hooks = nextHooks;
    }
  }
  writeFileSync(hooksJsonPath, `${JSON.stringify(hooksJson, null, 2)}\n`);
};

const transformCodexManifests = (outDir: string, version: string): void => {
  const pluginJsonPath = join(outDir, ".claude-plugin", "plugin.json");
  const plugin = JSON.parse(readFileSync(pluginJsonPath, "utf8")) as Record<string, unknown> & {
    keywords?: string[];
  };
  plugin.name = CODEX_PLUGIN_NAME;
  if (Array.isArray(plugin.keywords)) {
    plugin.keywords = plugin.keywords.map((keyword) =>
      keyword === "claude-code-plugin" ? "codex-plugin" : keyword,
    );
  }
  writeFileSync(pluginJsonPath, `${JSON.stringify(plugin, null, 2)}\n`);

  const marketplacePath = join(outDir, ".claude-plugin", "marketplace.json");
  if (existsSync(marketplacePath)) {
    const marketplace = JSON.parse(readFileSync(marketplacePath, "utf8")) as {
      plugins?: Array<{ name?: string; source?: string }>;
    };
    const bundledPlugin = marketplace.plugins?.[0];
    if (bundledPlugin) bundledPlugin.name = CODEX_PLUGIN_NAME;
    writeFileSync(marketplacePath, `${JSON.stringify(marketplace, null, 2)}\n`);
  }

  // KTD11: 미동봉 시 codex 가 자동 생성해 표시 메타 통제권을 잃어요. hooks 필드는
  // 넣지 않아요 (codex 스캐폴드 지침).
  const codexPluginDir = join(outDir, ".codex-plugin");
  mkdirSync(codexPluginDir, { recursive: true });
  const codexPlugin: Record<string, unknown> = { ...plugin, name: CODEX_PLUGIN_NAME, version };
  delete codexPlugin.hooks;
  writeFileSync(join(codexPluginDir, "plugin.json"), `${JSON.stringify(codexPlugin, null, 2)}\n`);
};

interface FrontmatterSplit {
  frontmatterLines: string[];
  body: string;
}

const splitFrontmatter = (source: string, file: string): FrontmatterSplit => {
  if (!source.startsWith("---\n")) throw new Error(`missing frontmatter in ${file}`);
  const end = source.indexOf("\n---\n", 4);
  if (end === -1) throw new Error(`unterminated frontmatter in ${file}`);
  return {
    frontmatterLines: source.slice(4, end).split("\n"),
    body: source.slice(end + "\n---\n".length),
  };
};

const isTopLevelKeyLine = (line: string): boolean => /^[A-Za-z_-]+:/.test(line);

// KTD7: codex 는 frontmatter examples 를 무시해요 — 재합성 description 은
// codex-overrides/routing/descriptions.json 이 소유하고, examples 블록은 카탈로그
// 예산을 위해 제거해요. KTD4 보조 수단인 절단 자기-복구 1줄도 여기서 prepend 해요.
const resynthesizeCodexDescriptions = (outDir: string): void => {
  const descriptionsPath = join(CODEX_OVERRIDES_DIR, "routing", "descriptions.json");
  const descriptions = existsSync(descriptionsPath)
    ? (JSON.parse(readFileSync(descriptionsPath, "utf8")) as Record<string, string>)
    : {};
  const skillsDir = join(outDir, "skills");
  if (!existsSync(skillsDir)) return;
  for (const skill of readdirSync(skillsDir)) {
    const skillPath = join(skillsDir, skill, "SKILL.md");
    if (!existsSync(skillPath)) continue;
    const source = readFileSync(skillPath, "utf8");
    const { frontmatterLines, body } = splitFrontmatter(source, skillPath);
    const nextLines: string[] = [];
    let index = 0;
    while (index < frontmatterLines.length) {
      const line = frontmatterLines[index];
      if (line.startsWith("description:")) {
        const override = descriptions[skill];
        const kept: string[] = [line];
        index += 1;
        while (index < frontmatterLines.length && !isTopLevelKeyLine(frontmatterLines[index])) {
          kept.push(frontmatterLines[index]);
          index += 1;
        }
        if (override) {
          nextLines.push(`description: ${JSON.stringify(override)}`);
        } else {
          nextLines.push(...kept);
        }
        continue;
      }
      if (line.startsWith("examples:")) {
        index += 1;
        while (index < frontmatterLines.length && !isTopLevelKeyLine(frontmatterLines[index])) {
          index += 1;
        }
        continue;
      }
      nextLines.push(line);
      index += 1;
    }
    const nextBody = body.startsWith(`${CODEX_RECOVERY_LINE}\n`)
      ? body
      : `${CODEX_RECOVERY_LINE}\n\n${body}`;
    writeFileSync(skillPath, `---\n${nextLines.join("\n")}\n---\n${nextBody}`);
  }
};

const buildBundle = ({ outDir, host }: Options): BundleStats => {
  assertSafeOutDir(outDir);
  rmSync(outDir, { recursive: true, force: true });
  mkdirSync(outDir, { recursive: true });

  for (const file of ROOT_FILES) {
    copyTree(join(REPO_ROOT, file), join(outDir, file));
  }

  for (const dir of ROOT_DIRS) {
    copyTree(join(REPO_ROOT, dir), join(outDir, dir));
  }

  if (host === "codex") {
    applyCodexSubstitutions(outDir);
    applyCodexOverrides(outDir);
    transformCodexHooks(outDir);
    const version = (
      JSON.parse(readFileSync(join(REPO_ROOT, ".claude-plugin", "plugin.json"), "utf8")) as {
        version: string;
      }
    ).version;
    transformCodexManifests(outDir, version);
    resynthesizeCodexDescriptions(outDir);
  }

  const pluginJson = join(outDir, ".claude-plugin", "plugin.json");
  if (!existsSync(pluginJson)) {
    throw new Error(`bundle is missing ${relative(outDir, pluginJson)}`);
  }

  const marketplaceJson = join(outDir, ".claude-plugin", "marketplace.json");
  if (existsSync(marketplaceJson)) {
    const marketplace = JSON.parse(readFileSync(marketplaceJson, "utf8")) as {
      plugins?: Array<{ source?: string }>;
    };
    const bundledPlugin = marketplace.plugins?.[0];
    if (!bundledPlugin) {
      throw new Error(`bundle marketplace is missing plugins[0]: ${relative(outDir, marketplaceJson)}`);
    }
    bundledPlugin.source = ".";
    writeFileSync(marketplaceJson, `${JSON.stringify(marketplace, null, 2)}\n`);
  }

  const stats = collectStats(outDir);
  return { outDir, host, ...stats };
};

const main = (): void => {
  const options = parseArgs();
  const stats = buildBundle(options);
  if (options.json) {
    console.log(JSON.stringify(stats));
  } else {
    console.log(
      `Built ${stats.host === "codex" ? "axhub-codex" : "axhub"} plugin bundle at ${stats.outDir} (${stats.files} files, ${stats.bytes} bytes)`,
    );
  }
};

if (import.meta.main) {
  main();
}
